//! Finds the matches of a query in a document's text, with byte offsets on
//! the *original* text (goal.md §5bis-B: highlighting stays exact with accents
//! and Arabic vowel marks), then builds snippets and preview lines.
//!
//! Exact matches are delimited with ⟦ ⟧, approximate ones (typo tolerance)
//! with ⟪ ⟫: the UI renders them differently (`ui/src/lib/text.ts`).
//! Extractors strip those characters from files.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Range;
use std::sync::OnceLock;

use rust_stemmers::{Algorithm, Stemmer};
use serde::Serialize;
use tantivy::tokenizer::{TokenStream, Tokenizer};

use crate::detect::{Detector, Detectors};
use crate::lang::{DocLang, Folding, WordTokenizer};
use crate::query::{Clause, ParsedQuery};

/// Scanning stops after this many bytes (huge logs, dumps).
const MAX_SCAN_BYTES: usize = 8 * 1024 * 1024;

fn stemmer(lang: DocLang) -> Option<&'static Stemmer> {
    static STEMMERS: OnceLock<HashMap<DocLang, Stemmer>> = OnceLock::new();
    STEMMERS
        .get_or_init(|| {
            HashMap::from([
                (DocLang::En, Stemmer::create(Algorithm::English)),
                (DocLang::Fr, Stemmer::create(Algorithm::French)),
                (DocLang::Es, Stemmer::create(Algorithm::Spanish)),
                (DocLang::Ar, Stemmer::create(Algorithm::Arabic)),
            ])
        })
        .get(&lang)
}

/// Stem of a raw word, with the same input normalization as the index.
fn stem(lang: DocLang, raw: &str) -> Option<String> {
    let input: String = raw
        .chars()
        .filter(|c| !matches!(c, '\u{0610}'..='\u{061A}' | '\u{064B}'..='\u{065F}' | '\u{0670}' | '\u{0640}'))
        .flat_map(char::to_lowercase)
        .collect();
    stemmer(lang).map(|s| s.stem(&input).into_owned())
}

struct Tok {
    folded: String,
    range: Range<usize>,
}

fn tokenize(text: &str) -> Vec<Tok> {
    let mut tokenizer = WordTokenizer::new(Folding::Full);
    let mut stream = tokenizer.token_stream(text);
    let mut out = Vec::new();
    while stream.advance() {
        let t = stream.token();
        out.push(Tok { folded: t.text.clone(), range: t.offset_from..t.offset_to });
    }
    out
}

/// Maximum edit distance allowed for a word of this length.
///
/// Short words get no tolerance: one different letter usually makes another
/// real word ("alger" → "alter", "prix" → "pris"), not a typo (BUG-019).
pub fn fuzzy_distance(word: &str) -> u8 {
    match word.chars().count() {
        0..=5 => 0,
        6..=9 => 1,
        _ => 2,
    }
}

/// One match: its byte range in the original text, and whether it only
/// matched through typo tolerance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub range: Range<usize>,
    pub fuzzy: bool,
}

fn levenshtein_within(a: &str, b: &str, max: usize) -> bool {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > max {
        return false;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        let mut row_min = cur[0];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + usize::from(ca != cb));
            row_min = row_min.min(cur[j + 1]);
        }
        if row_min > max {
            return false;
        }
        prev = cur;
    }
    prev[b.len()] <= max
}

/// Arabic prefixes (article + attached conjunction/preposition) removed
/// before comparing word lengths: "العقد" is the word "عقد" plus an article.
const ARABIC_PREFIXES: [&str; 6] = ["وال", "بال", "كال", "فال", "لل", "ال"];

/// Length of a folded word without its Arabic article, in characters.
fn core_len(folded: &str) -> usize {
    let n = folded.chars().count();
    ARABIC_PREFIXES
        .iter()
        .find_map(|p| folded.strip_prefix(p).map(str::chars).map(Iterator::count).filter(|rest| *rest >= 2))
        .unwrap_or(n)
}

/// Two words with the same stem are the same word only when they differ by
/// one letter at most (plural, feminine): "contrat"/"contrats" yes,
/// "Alger"/"Algérie" no (BUG-020). Derived words are not grouped.
const MAX_INFLECTION_LEN_DIFF: usize = 1;

/// Search options of the UI chips (Aa, ab, .*, typo tolerance).
#[derive(Clone, Copy, Debug)]
pub struct MatchOptions {
    pub fuzzy: bool,
    /// `Aa`: only occurrences with the exact case (and accents) count.
    pub case_sensitive: bool,
    /// `ab`: whole words only; off = the word may be part of a longer one.
    pub whole_word: bool,
}

impl Default for MatchOptions {
    fn default() -> Self {
        Self { fuzzy: false, case_sensitive: false, whole_word: true }
    }
}

/// Regular expressions above this compiled size are refused.
const MAX_REGEX_BYTES: usize = 10 * 1024 * 1024;

/// What to look for in a text, prepared once per query.
pub struct Matcher {
    words: HashSet<String>,
    /// Query words as typed (case-sensitive mode).
    raw_words: HashSet<String>,
    /// Per language: stem → lengths of the query words that produced it.
    stems: HashMap<DocLang, HashMap<String, Vec<usize>>>,
    fuzzy: Vec<(String, usize)>,
    /// (folded words, phrase as typed)
    phrases: Vec<(Vec<String>, String)>,
    regexes: Vec<regex::Regex>,
    /// Detectors (lot 6.2): validated data.
    detectors: Vec<Detector>,
    /// `a NEAR b` groups: one matcher per term, and the distance in characters.
    near: Vec<(Vec<Matcher>, usize)>,
    /// `LINES:a-b`: only matches inside these lines (1-based, inclusive).
    lines: Option<(usize, usize)>,
    options: MatchOptions,
}

impl Matcher {
    /// Fails only on an invalid regular expression.
    pub fn new(query: &ParsedQuery, options: MatchOptions) -> Result<Self, regex::Error> {
        let mut matcher = Matcher {
            words: HashSet::new(),
            raw_words: HashSet::new(),
            stems: HashMap::new(),
            fuzzy: Vec::new(),
            phrases: Vec::new(),
            regexes: Vec::new(),
            detectors: Vec::new(),
            near: Vec::new(),
            lines: query.lines,
            options,
        };
        for clause in query.positive_clauses() {
            // LIKE: this word with typo tolerance, whatever the option.
            let like = matches!(clause, Clause::Like(_));
            let raw = match clause {
                Clause::Word(w) | Clause::Phrase(w) | Clause::Like(w) => w.as_str(),
                Clause::Near { terms, chars } => {
                    let parts = terms
                        .iter()
                        .map(|t| Matcher::new(&ParsedQuery { must: vec![vec![t.clone()]], ..Default::default() }, options))
                        .collect::<Result<Vec<_>, _>>()?;
                    matcher.near.push((parts, *chars));
                    continue;
                }
                Clause::Detector(detector) => {
                    matcher.detectors.push(*detector);
                    continue;
                }
                Clause::Regex(pattern) => {
                    matcher.regexes.push(
                        regex::RegexBuilder::new(pattern)
                            .case_insensitive(!options.case_sensitive)
                            .size_limit(MAX_REGEX_BYTES)
                            .build()?,
                    );
                    continue;
                }
            };
            let folded: Vec<String> = tokenize(raw).into_iter().map(|t| t.folded).collect();
            // A phrase, or a word the tokenizer splits ("facture_2024-117").
            if folded.len() > 1 {
                matcher.phrases.push((folded, raw.trim().to_owned()));
                continue;
            }
            let Some(word) = folded.into_iter().next() else { continue };
            // Case-sensitive: exact spelling only, no word forms, no typos.
            if !options.case_sensitive {
                let len = core_len(&word);
                for lang in DocLang::STEMMED {
                    if let Some(s) = stem(lang, raw) {
                        matcher.stems.entry(lang).or_default().entry(s).or_default().push(len);
                    }
                }
                if options.fuzzy || like {
                    let d = usize::from(fuzzy_distance(&word)).max(usize::from(like));
                    if d > 0 {
                        matcher.fuzzy.push((word.clone(), d));
                    }
                }
            }
            matcher.raw_words.insert(raw.trim().to_owned());
            matcher.words.insert(word);
        }
        Ok(matcher)
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
            && self.phrases.is_empty()
            && self.regexes.is_empty()
            && self.near.is_empty()
            && self.detectors.is_empty()
    }

    /// Occurrences per detector code (lot 6.2); empty without detectors.
    pub fn detections(&self, text: &str) -> BTreeMap<String, usize> {
        if self.detectors.is_empty() {
            return BTreeMap::new();
        }
        Detectors::new(&self.detectors).count(&text[..floor_char_boundary(text, MAX_SCAN_BYTES)])
    }

    fn word_hit(&self, folded: &str, raw: &str) -> bool {
        if self.options.case_sensitive {
            return if self.options.whole_word {
                self.raw_words.contains(raw)
            } else {
                self.raw_words.iter().any(|w| raw.contains(w.as_str()))
            };
        }
        if self.options.whole_word {
            self.words.contains(folded)
        } else {
            self.words.iter().any(|w| folded.contains(w.as_str()))
        }
    }

    /// A word of the text: `Some(false)` exact (or a form of a searched word),
    /// `Some(true)` approximate (typo tolerance), `None` no match.
    fn judge(
        &self,
        tok: &Tok,
        raw: &str,
        lang: DocLang,
        stems: Option<&HashMap<String, Vec<usize>>>,
        stem_lens: &[usize],
    ) -> Option<bool> {
        if self.word_hit(&tok.folded, raw) {
            return Some(false);
        }
        if let Some(map) = stems {
            let tok_len = core_len(&tok.folded);
            let near = |q: &usize| tok_len.abs_diff(*q) <= MAX_INFLECTION_LEN_DIFF;
            if stem_lens.iter().any(near) && stem(lang, raw).and_then(|s| map.get(&s)).is_some_and(|lens| lens.iter().any(near)) {
                return Some(false);
            }
        }
        self.fuzzy.iter().any(|(q, d)| levenshtein_within(&tok.folded, q, *d)).then_some(true)
    }

    /// Non-overlapping matches, sorted by position.
    pub fn find(&self, text: &str, lang: DocLang) -> Vec<Match> {
        let scan = &text[..floor_char_boundary(text, MAX_SCAN_BYTES)];
        // `Aa`: a word or phrase matches only where it is spelled as typed, so
        // a text without that spelling is skipped without being split in words.
        let exact_only = self.options.case_sensitive && self.regexes.is_empty() && self.near.is_empty() && self.detectors.is_empty();
        if exact_only
            && !self.raw_words.iter().chain(self.phrases.iter().map(|(_, raw)| raw)).any(|w| scan.contains(w.as_str()))
        {
            return Vec::new();
        }
        let tokens = tokenize(scan);
        let stems = self.stems.get(&lang).filter(|s| !s.is_empty());
        let mut covered = vec![false; tokens.len()];
        let mut ranges = Vec::new();

        for (phrase, raw_phrase) in &self.phrases {
            let n = phrase.len();
            if n == 0 || tokens.len() < n {
                continue;
            }
            for i in 0..=tokens.len() - n {
                if covered[i..i + n].iter().any(|&c| c) {
                    continue;
                }
                let same_words = tokens[i..i + n].iter().zip(phrase).all(|(t, p)| t.folded == *p);
                let same_case = !self.options.case_sensitive
                    || scan[tokens[i].range.start..tokens[i + n - 1].range.end] == **raw_phrase;
                if same_words && same_case {
                    ranges.push(Match { range: tokens[i].range.start..tokens[i + n - 1].range.end, fuzzy: false });
                    covered[i..i + n].iter_mut().for_each(|c| *c = true);
                }
            }
        }

        // Lengths a word form may have: a word too far from all of them is
        // not stemmed (the costly part on long texts).
        let stem_lens: Vec<usize> = stems.map(|map| map.values().flatten().copied().collect()).unwrap_or_default();
        // Code repeats the same identifiers: each spelling is judged once per text.
        let mut judged: HashMap<&str, Option<bool>> = HashMap::new();
        for (i, tok) in tokens.iter().enumerate() {
            if covered[i] {
                continue;
            }
            let raw = &scan[tok.range.clone()];
            let verdict = *judged.entry(raw).or_insert_with(|| self.judge(tok, raw, lang, stems, &stem_lens));
            if let Some(fuzzy) = verdict {
                ranges.push(Match { range: tok.range.clone(), fuzzy });
            }
        }
        for re in &self.regexes {
            ranges.extend(
                re.find_iter(scan).filter(|m| !m.is_empty()).map(|m| Match { range: m.range(), fuzzy: false }),
            );
        }
        for (parts, chars) in &self.near {
            ranges.extend(near_matches(scan, parts, *chars, lang));
        }
        if !self.detectors.is_empty() {
            ranges.extend(Detectors::new(&self.detectors).find(scan).into_iter().map(|(_, range)| Match { range, fuzzy: false }));
        }
        // LINES: only what falls inside the requested lines.
        if let Some((first, last)) = self.lines {
            let starts = line_starts(scan);
            let begin = starts.get(first - 1).copied().unwrap_or(scan.len());
            let end = starts.get(last).copied().unwrap_or(scan.len());
            ranges.retain(|m| m.range.start >= begin && m.range.end <= end);
        }
        // Word and regex matches may overlap: keep the first of each overlap.
        ranges.sort_by_key(|m| (m.range.start, std::cmp::Reverse(m.range.end)));
        let mut merged: Vec<Match> = Vec::with_capacity(ranges.len());
        for m in ranges {
            if merged.last().is_some_and(|last| m.range.start < last.range.end) {
                continue;
            }
            merged.push(m);
        }
        merged
    }
}

/// The occurrences of each NEAR term that have an occurrence of every other
/// term within `chars` characters (counted as characters, not bytes: an
/// Arabic letter counts once).
fn near_matches(text: &str, parts: &[Matcher], chars: usize, lang: DocLang) -> Vec<Match> {
    let found: Vec<Vec<Match>> = parts.iter().map(|p| p.find(text, lang)).collect();
    if found.iter().any(Vec::is_empty) {
        return Vec::new();
    }
    let gap = |a: &Match, b: &Match| -> usize {
        let (from, to) = if a.range.end <= b.range.start {
            (a.range.end, b.range.start)
        } else if b.range.end <= a.range.start {
            (b.range.end, a.range.start)
        } else {
            return 0;
        };
        // A UTF-8 character is at most 4 bytes: far enough in bytes is far enough.
        if to - from > chars * 4 {
            return usize::MAX;
        }
        text[from..to].chars().count()
    };
    let mut out = Vec::new();
    for (k, matches) in found.iter().enumerate() {
        for m in matches {
            let close = found.iter().enumerate().filter(|(j, _)| *j != k).all(|(_, others)| others.iter().any(|o| gap(m, o) <= chars));
            if close {
                out.push(m.clone());
            }
        }
    }
    out
}

fn floor_char_boundary(text: &str, index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    let mut i = index;
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn ceil_char_boundary(text: &str, index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    let mut i = index;
    while !text.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// Byte offset where each line starts.
fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0).chain(text.match_indices('\n').map(|(i, _)| i + 1)).collect()
}

/// Inserts ⟦ ⟧ (exact) or ⟪ ⟫ (approximate) around the matches that fall
/// inside `window` (offsets are absolute in `text`).
fn marked(text: &str, window: Range<usize>, matches: &[Match]) -> String {
    let mut out = String::with_capacity(window.len() + 16);
    let mut cursor = window.start;
    for m in matches {
        let start = m.range.start.max(window.start);
        let end = m.range.end.min(window.end);
        if start >= end || start < cursor {
            continue;
        }
        let (open, close) = if m.fuzzy { ('⟪', '⟫') } else { ('⟦', '⟧') };
        out.push_str(&text[cursor..start]);
        out.push(open);
        out.push_str(&text[start..end]);
        out.push(close);
        cursor = end;
    }
    out.push_str(&text[cursor..window.end]);
    out
}

#[derive(Clone, Debug, Serialize)]
pub struct Snippet {
    /// 1-based line number.
    pub line: usize,
    pub text: String,
}

/// Up to `max` one-line snippets on distinct lines, around exact matches
/// first, then approximate ones.
pub fn snippets(text: &str, matches: &[Match], max: usize) -> Vec<Snippet> {
    const LONG_LINE: usize = 240;
    let starts = line_starts(text);
    let mut out: Vec<Snippet> = Vec::new();
    let ordered = matches.iter().filter(|m| !m.fuzzy).chain(matches.iter().filter(|m| m.fuzzy));
    for m in ordered {
        let r = &m.range;
        if out.len() >= max {
            break;
        }
        let line_idx = starts.partition_point(|&s| s <= r.start) - 1;
        if out.iter().any(|s| s.line == line_idx + 1) {
            continue;
        }
        let line_start = starts[line_idx];
        let line_end = starts.get(line_idx + 1).map_or(text.len(), |&n| n - 1);
        let (start, end) = if line_end - line_start > LONG_LINE {
            (
                floor_char_boundary(text, r.start.saturating_sub(100).max(line_start)),
                ceil_char_boundary(text, (r.end + 140).min(line_end)),
            )
        } else {
            (line_start, line_end)
        };
        let mut snippet = marked(text, start..end, matches).trim().replace('\t', " ").trim_end_matches('\r').to_owned();
        if start > line_start {
            snippet.insert(0, '…');
        }
        if end < line_end {
            snippet.push('…');
        }
        out.push(Snippet { line: line_idx + 1, text: snippet });
    }
    out.sort_by_key(|s| s.line);
    out
}

#[derive(Clone, Debug, Serialize)]
pub struct PreviewLine {
    pub n: usize,
    pub text: String,
}

/// A section title written by the extractors: `— Sheet1 —`, `— 3 —`.
fn is_section_header(line: &str) -> bool {
    let line = line.trim_end_matches('\r');
    line.len() > 4 && line.starts_with("— ") && line.ends_with(" —")
}

/// Preview lines: the whole document when short, otherwise the start plus
/// every match with 2 lines of context. Very long lines are cut.
/// `sections`: sheets / slides; a kept line always comes with the title of
/// its section, so the preview can tell which sheet or slide it is on.
pub fn preview_lines(text: &str, matches: &[Match], sections: bool) -> (Vec<PreviewLine>, bool) {
    const MAX_LINES: usize = 400;
    const MAX_LINE_BYTES: usize = 2000;
    const CONTEXT: usize = 2;
    let starts = line_starts(text);
    let total = starts.len();

    let mut wanted: Vec<usize> = if total <= MAX_LINES {
        (0..total).collect()
    } else {
        let mut set: Vec<usize> = (0..5.min(total)).collect();
        for m in matches {
            let idx = starts.partition_point(|&s| s <= m.range.start) - 1;
            set.extend(idx.saturating_sub(CONTEXT)..(idx + CONTEXT + 1).min(total));
        }
        if sections {
            let headers: Vec<usize> = (0..total)
                .filter(|&i| is_section_header(&text[starts[i]..starts.get(i + 1).map_or(text.len(), |&n| n - 1)]))
                .collect();
            let titles: Vec<usize> = set
                .iter()
                .filter_map(|&idx| headers.get(headers.partition_point(|&h| h <= idx).checked_sub(1)?).copied())
                .collect();
            set.extend(titles);
        }
        set.sort_unstable();
        set.dedup();
        set
    };
    let truncated = wanted.len() > MAX_LINES || total > MAX_LINES;
    wanted.truncate(MAX_LINES);

    let lines = wanted
        .into_iter()
        .map(|idx| {
            let start = starts[idx];
            let end = starts.get(idx + 1).map_or(text.len(), |&n| n - 1);
            let end = end.min(ceil_char_boundary(text, start + MAX_LINE_BYTES));
            PreviewLine { n: idx + 1, text: marked(text, start..end, matches).trim_end_matches('\r').to_owned() }
        })
        .collect();
    (lines, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::parse;

    fn hits(query: &str, fuzzy: bool, text: &str, lang: DocLang) -> Vec<String> {
        hits_with(query, MatchOptions { fuzzy, ..Default::default() }, text, lang)
    }

    fn hits_with(query: &str, options: MatchOptions, text: &str, lang: DocLang) -> Vec<String> {
        let m = Matcher::new(&parse(query), options).unwrap();
        m.find(text, lang).into_iter().map(|m| text[m.range].to_owned()).collect()
    }

    #[test]
    fn accents_and_arabic_vowels_are_ignored() {
        assert_eq!(hits("resume", false, "Voici le Résumé.", DocLang::Fr), ["Résumé"]);
        // The Arabic stemmer also removes the article: "عقد" finds "العَقْدُ".
        assert_eq!(hits("عقد", false, "هذا العَقْدُ وهذا عقد", DocLang::Ar), ["العَقْدُ", "عقد"]);
        assert_eq!(hits("العقد", false, "هذا العَقْدُ مهم", DocLang::Ar), ["العَقْدُ"]);
    }

    #[test]
    fn stems_find_word_forms_in_the_document_language() {
        assert_eq!(hits("contrat", false, "Les contrats signés.", DocLang::Fr), ["contrats"]);
        assert_eq!(hits("contrato", false, "Los contratos firmados.", DocLang::Es), ["contratos"]);
    }

    #[test]
    fn fuzzy_tolerates_typos() {
        assert_eq!(hits("contrat", true, "export class Contract {}", DocLang::En), ["Contract"]);
        assert!(hits("contrat", false, "export class Contract {}", DocLang::En).is_empty());
    }

    #[test]
    fn stems_group_inflections_not_other_words() {
        // BUG-020: same French stem, but a different word.
        assert_eq!(hits("alger", false, "Alger, Algérie", DocLang::Fr), ["Alger"]);
        assert_eq!(hits("signé", false, "signée et signés", DocLang::Fr), ["signée", "signés"]);
        assert_eq!(hits("عقد", false, "وقّع العَقْدَ", DocLang::Ar), ["العَقْدَ"]);
    }

    #[test]
    fn case_whole_word_and_regex_options() {
        let text = "Alger, ALGER et sous-contrats. Facture INV-2024-117 et inv-2023-001.";
        let case = MatchOptions { case_sensitive: true, ..Default::default() };
        assert_eq!(hits_with("Alger", case, text, DocLang::Fr), ["Alger"]);
        assert_eq!(hits("alger", false, text, DocLang::Fr), ["Alger", "ALGER"]);
        let partial = MatchOptions { whole_word: false, ..Default::default() };
        assert_eq!(hits_with("contrat", partial, text, DocLang::Fr), ["contrats"]);
        assert_eq!(hits(r"/INV-\d{4}-\d{3}/", false, text, DocLang::Fr), ["INV-2024-117", "inv-2023-001"]);
        assert_eq!(hits_with(r"/INV-\d{4}/", case, text, DocLang::Fr), ["INV-2024"]);
        assert!(Matcher::new(&parse("/(unclosed/"), MatchOptions::default()).is_err());
    }

    #[test]
    fn short_words_get_no_typo_tolerance() {
        // BUG-019: "alger" (the city) must not find "alter".
        assert!(hits("alger", true, "please alter the requests", DocLang::En).is_empty());
        assert_eq!(hits("alger", true, "Vol pour Alger demain", DocLang::Fr), ["Alger"]);
    }

    #[test]
    fn approximate_matches_are_flagged_and_marked_apart() {
        let text = "Le paiement du contrat. Le paiemant est en retard.";
        let m = Matcher::new(&parse("paiement"), MatchOptions { fuzzy: true, ..Default::default() }).unwrap();
        let found = m.find(text, DocLang::Fr);
        assert_eq!(found.iter().map(|x| x.fuzzy).collect::<Vec<_>>(), [false, true]);
        let s = snippets(text, &found, 1);
        assert_eq!(s[0].text, "Le ⟦paiement⟧ du contrat. Le ⟪paiemant⟫ est en retard.");
    }

    #[test]
    fn phrases_match_consecutive_words() {
        assert_eq!(
            hits("\"clause de confidentialité\"", false, "Voir la Clause de Confidentialité ci-dessous.", DocLang::Fr),
            ["Clause de Confidentialité"]
        );
    }

    #[test]
    fn snippets_mark_matches_with_line_numbers() {
        let text = "Titre\n\nLe présent contrat prend effet.\nFin du contrat.";
        let m = Matcher::new(&parse("contrat"), MatchOptions::default()).unwrap();
        let s = snippets(text, &m.find(text, DocLang::Fr), 2);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].line, 3);
        assert_eq!(s[0].text, "Le présent ⟦contrat⟧ prend effet.");
        assert_eq!(s[1].line, 4);
    }

    #[test]
    fn near_keeps_only_close_occurrences_in_characters() {
        let text = "Le contrat prévoit la résiliation. Plus loin, bien plus loin, un autre contrat.\nعقد ثم فسخ";
        let far = format!("{text} {}", "x ".repeat(80));
        let m = Matcher::new(&parse("contrat NEAR:30 résiliation"), MatchOptions::default()).unwrap();
        let found: Vec<&str> = m.find(&far, DocLang::Fr).into_iter().map(|x| &far[x.range]).collect();
        assert_eq!(found, ["contrat", "résiliation"], "the second 'contrat' is too far");
        // Characters, not bytes: 5 Arabic characters apart.
        let ar = Matcher::new(&parse("عقد NEAR:5 فسخ"), MatchOptions::default()).unwrap();
        assert_eq!(ar.find(text, DocLang::Ar).len(), 2);
        let too_close = Matcher::new(&parse("عقد NEAR:3 فسخ"), MatchOptions::default()).unwrap();
        assert!(too_close.find(text, DocLang::Ar).is_empty());
    }

    #[test]
    fn like_and_lines() {
        let like = Matcher::new(&parse("LIKE necessary"), MatchOptions::default()).unwrap();
        assert_eq!(like.find("it is necesary", DocLang::En).len(), 1, "typo tolerated for LIKE only");
        let lines = Matcher::new(&parse("LINES:2-3 tower"), MatchOptions::default()).unwrap();
        let text = "tower one\ntwo tower\nthree tower\nfour tower";
        let found: Vec<usize> = lines.find(text, DocLang::En).into_iter().map(|m| m.range.start).collect();
        assert_eq!(found, [14, 26], "only lines 2 and 3");
    }

    #[test]
    fn long_sheets_keep_the_title_of_each_match() {
        let mut text = String::from("— Janvier —\n");
        for i in 0..500 {
            text.push_str(&format!("ligne {i}\t{i}\n"));
        }
        text.push_str("— Février —\n");
        for i in 0..500 {
            text.push_str(&format!("{}\t{i}\n", if i == 300 { "contrat" } else { "ligne" }));
        }
        let m = Matcher::new(&parse("contrat"), MatchOptions::default()).unwrap();
        let matches = m.find(&text, DocLang::Fr);
        let (lines, truncated) = preview_lines(&text, &matches, true);
        assert!(truncated);
        let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        let feb = texts.iter().position(|t| *t == "— Février —").expect("sheet title kept");
        let hit = texts.iter().position(|t| t.contains("⟦contrat⟧")).unwrap();
        assert!(feb < hit);
        // Without sections, the title far above the match is not kept.
        let (plain, _) = preview_lines(&text, &matches, false);
        assert!(!plain.iter().any(|l| l.text == "— Février —"));
    }
}
