//! Query building and execution on one site index.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Bound;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tantivy::collector::{Count, TopDocs};
use tantivy::query::{
    AllQuery, BooleanQuery, BoostQuery, FuzzyTermQuery, Occur, PhraseQuery, Query, RangeQuery, TermQuery, TermSetQuery,
};
use tantivy::schema::{IndexRecordOption, Value};
use rayon::prelude::*;
use tantivy::{DocAddress, TantivyDocument, Term};

use crate::detect::Detector;
use crate::facets::{date_field_name, facet_counts, Facets};
use crate::disk::{attribute_mask, in_range, Digest, DiskCheck};
use crate::error::Result;
use crate::extract::split_inner;
use crate::highlight::{fuzzy_distance, snippets, MatchOptions, Matcher, Snippet};
use crate::index::{Fields, SiteIndex};
use crate::lang::{analyze, generic_analyzer, stem_analyzer, DocLang};
use crate::error::CoreError;
use crate::names::NamePattern;
use crate::query::{parse, Clause, ParsedQuery};

/// Search request coming from the UI (serde camelCase).
/// Also stored as is by the alerts (lot 6.1), hence `Serialize`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchRequest {
    pub query: String,
    pub fuzzy: bool,
    /// `Aa` chip.
    pub case_sensitive: bool,
    /// `ab` chip (on by default).
    pub whole_word: bool,
    /// `.*` chip: the whole input is one regular expression.
    pub regex: bool,
    /// File families (`pdf`, `word`…); empty = all.
    pub kinds: Vec<String>,
    /// Document languages (`fr`, `ar`…); empty = all.
    pub langs: Vec<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    /// The date the range applies to (lot 5.8).
    pub date_field: DateField,
    /// Unix seconds, `[date_from, date_to)`.
    pub date_from: Option<u64>,
    pub date_to: Option<u64>,
    /// `readOnly`, `hidden`, `system`: all required, read on the disk.
    pub attributes: Vec<String>,
    /// MD5 or SHA-256 of the file, computed on the disk ("find copies").
    pub hash: String,
    /// File-name criterion (`*.pdf; facture-*`, `/regex/`), see names.rs.
    pub name_pattern: String,
    /// Look for folders (by name) instead of files.
    pub folders: bool,
    /// "Search within these results" (lot 5.6): only these paths (inner
    /// documents as `archive.zip › entry`); empty = everywhere.
    pub within_paths: Vec<String>,
    /// "Search with Prospector" on a folder (lot 5.7): only what is inside
    /// it, see scope.rs.
    pub in_folder: Option<String>,
    /// Only the documents of these files on disk (alerts, lot 6.1: the files
    /// a batch of changes touched); empty = all.
    pub in_files: Vec<String>,
    /// Detectors (lot 6.2): documents holding at least one of these data.
    pub detectors: Vec<Detector>,
    /// Also count per kind, language, year and site (lot 6.3).
    pub facets: bool,
    /// Terms read from a file (lot 7.4): documents with at least one of
    /// them, or all of them (`term_list_all`), besides the query.
    pub term_list: Vec<String>,
    pub term_list_all: bool,
    /// Also by meaning (Étape 8, lot 8.3): the app computes the vector of
    /// the question and calls `Engine::search_meaning`.
    pub meaning: bool,
    pub limit: Option<usize>,
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            fuzzy: false,
            case_sensitive: false,
            whole_word: true,
            regex: false,
            kinds: Vec::new(),
            langs: Vec::new(),
            min_size: None,
            max_size: None,
            date_field: DateField::Modified,
            date_from: None,
            date_to: None,
            attributes: Vec::new(),
            hash: String::new(),
            name_pattern: String::new(),
            folders: false,
            within_paths: Vec::new(),
            in_folder: None,
            in_files: Vec::new(),
            detectors: Vec::new(),
            facets: false,
            term_list: Vec::new(),
            term_list_all: false,
            meaning: false,
            limit: None,
        }
    }
}

impl SearchRequest {
    /// The query, parsed according to the `.*` option.
    pub fn parsed(&self) -> ParsedQuery {
        let mut parsed = if self.regex { ParsedQuery::whole_regex(&self.query) } else { parse(&self.query) };
        // The detectors are one more AND group: any of them.
        if !self.detectors.is_empty() {
            parsed.must.push(self.detectors.iter().map(|d| Clause::Detector(*d)).collect());
        }
        // A term list (lot 7.4): one more group (any of them), or one per term.
        let terms = self.term_list.iter().map(|t| crate::terms::clause(t));
        if self.term_list_all {
            parsed.must.extend(terms.map(|c| vec![c]));
        } else if !self.term_list.is_empty() {
            parsed.must.push(terms.collect());
        }
        parsed
    }

    pub fn match_options(&self) -> MatchOptions {
        MatchOptions { fuzzy: self.fuzzy, case_sensitive: self.case_sensitive, whole_word: self.whole_word }
    }

    /// The file-name criterion, if any (fails on an invalid regex).
    pub fn names(&self) -> Result<Option<NamePattern>> {
        NamePattern::parse(&self.name_pattern)
    }

    /// Nothing to look for: no text, no file name, no digest.
    pub fn is_empty(&self) -> bool {
        self.parsed().is_empty() && self.name_pattern.trim().is_empty() && self.hash.trim().is_empty() && self.detectors.is_empty()
    }

    /// The date range on the modification or creation date (the index and
    /// the crawl know both); the last access is checked on the disk.
    pub fn date_ok(&self, modified: u64, created: u64) -> bool {
        match self.date_field {
            DateField::Modified => in_range(modified, self.date_from, self.date_to),
            DateField::Created => in_range(created, self.date_from, self.date_to),
            DateField::Accessed => true,
        }
    }

    /// What must be read on the disk (last access, attributes, digest);
    /// `None` when nothing. Fails on an invalid digest.
    pub fn disk_check(&self) -> Result<Option<DiskCheck>> {
        let dated = self.date_from.is_some() || self.date_to.is_some();
        let check = DiskCheck {
            accessed: (self.date_field == DateField::Accessed && dated).then_some((self.date_from, self.date_to)),
            attributes: attribute_mask(&self.attributes),
            digest: Digest::parse(&self.hash)?,
        };
        Ok((!check.is_empty()).then_some(check))
    }

    /// The paths of "search within results", if any.
    pub fn within(&self) -> Option<HashSet<&str>> {
        (!self.within_paths.is_empty()).then(|| self.within_paths.iter().map(String::as_str).collect())
    }

    /// The folder the search is limited to, if any.
    pub fn folder(&self) -> Option<&str> {
        self.in_folder.as_deref().map(str::trim).filter(|f| !f.is_empty())
    }

    /// Candidates of the index are checked on their text: what the index
    /// cannot decide (regex, NEAR, LINES, detectors) or the exact case (`Aa`).
    pub fn verified(&self, parsed: &ParsedQuery) -> bool {
        parsed.needs_verification() || self.case_sensitive
    }

    /// The highlighter of this request (fails on an invalid regex).
    pub fn matcher(&self, parsed: &ParsedQuery) -> Result<Matcher> {
        Matcher::new(parsed, self.match_options()).map_err(|e| CoreError::InvalidQuery { message: e.to_string() })
    }
}

/// What the detectors found, for one kind of data (lot 6.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionTotal {
    pub files: usize,
    pub matches: usize,
}

/// Per detector code.
pub type DetectionTotals = BTreeMap<String, DetectionTotal>;

pub fn add_detections(totals: &mut DetectionTotals, found: &BTreeMap<String, usize>) {
    for (code, &count) in found {
        let total = totals.entry(code.clone()).or_default();
        total.files += 1;
        total.matches += count;
    }
}

/// Which date a date range applies to.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DateField {
    #[default]
    Modified,
    Created,
    /// Last access: read on the disk, never indexed.
    Accessed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub site_id: String,
    pub path: String,
    pub kind: String,
    pub lang: String,
    pub size_bytes: u64,
    /// Unix seconds.
    pub modified: u64,
    /// Unix seconds (lot 5.8).
    pub created: u64,
    /// Occurrences per detector (lot 6.2); empty without detectors.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub detections: BTreeMap<String, usize>,
    pub match_count: usize,
    /// Matches found without typo tolerance. 0 = the file only matched
    /// approximately (shown after the others, marked as such).
    pub exact_count: usize,
    pub score: f32,
    pub snippets: Vec<Snippet>,
    /// Family of a file inside an archive or an e-mail (lot 6.7: it can be
    /// extracted; an image gets its picture). None on disk and for messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_kind: Option<String>,
    /// Found by its meaning too (Étape 8, lot 8.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meaning: Option<MeaningMatch>,
}

/// How a document answers the meaning of the search (lot 8.3).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeaningMatch {
    /// Similarity of its closest passage (1 = same meaning).
    pub score: f32,
    /// That passage in the document's text (bytes).
    pub start: usize,
    pub end: usize,
    /// Found by its meaning only (no word of the search in it).
    pub only: bool,
}

impl Hit {
    /// Fills [`Hit::inner_kind`] from the path.
    pub fn with_inner_kind(mut self) -> Self {
        self.inner_kind = crate::unpack::inner_kind(&self.path).map(|k| k.as_str().to_owned());
        self
    }
}

const DEFAULT_LIMIT: usize = 200;

/// When candidates are checked on their text (regex, NEAR, LINES), at most
/// this many are examined.
const VERIFIED_CANDIDATES: usize = 100_000;

/// Candidates read and checked together, in parallel.
const VERIFY_BATCH: usize = 256;

/// Score multiplier of files that only match approximately.
const APPROXIMATE_PENALTY: f32 = 0.2;

/// `ab` off: the indexed words that contain each searched word. Walking the
/// dictionary is the costly part (millions of words in code): it is done once
/// per site and search, and reused by the counts per criterion.
#[derive(Default)]
pub(crate) struct Contains(HashMap<String, Vec<String>>);

impl Contains {
    pub(crate) fn expand(index: &SiteIndex, parsed: &ParsedQuery, req: &SearchRequest) -> Result<Self> {
        fn words<'a>(clause: &'a Clause, out: &mut Vec<&'a str>) {
            match clause {
                Clause::Word(w) | Clause::Like(w) => out.push(w),
                Clause::Near { terms, .. } => terms.iter().for_each(|t| words(t, out)),
                _ => {}
            }
        }
        if req.whole_word {
            return Ok(Self::default());
        }
        let mut raw = Vec::new();
        parsed.must.iter().flatten().chain(&parsed.must_not).for_each(|c| words(c, &mut raw));
        let tokens: Vec<String> = raw
            .into_iter()
            .filter_map(|w| {
                let folded = analyze(&mut generic_analyzer(), w);
                (folded.len() == 1).then(|| folded.into_iter().next()).flatten()
            })
            .collect();
        if tokens.is_empty() {
            return Ok(Self::default());
        }
        let mut found: HashMap<String, HashSet<String>> = tokens.iter().map(|t| (t.clone(), HashSet::new())).collect();
        for segment in index.reader.searcher().segment_readers() {
            let inverted = segment.inverted_index(index.fields.body)?;
            let mut stream = inverted.terms().stream()?;
            while stream.advance() {
                let Ok(term) = std::str::from_utf8(stream.key()) else { continue };
                for token in tokens.iter().filter(|t| term.contains(String::as_str(t))) {
                    if let Some(set) = found.get_mut(token) {
                        set.insert(term.to_owned());
                    }
                }
            }
        }
        Ok(Self(found.into_iter().map(|(k, v)| (k, v.into_iter().collect())).collect()))
    }
}

/// How the words of a query are looked up in the index.
#[derive(Clone, Copy)]
pub(crate) struct WordMode<'a> {
    fuzzy: bool,
    /// Word forms (stems) and typos; not with `Aa` (exact spelling only).
    forms: bool,
    /// `ab` off: the indexed words containing each word.
    contains: &'a Contains,
}

/// One word: accent-free `body`, file `name` (boosted), stemmed fields, typos.
fn word_query(f: &Fields, word: &str, mode: WordMode) -> Option<Box<dyn Query>> {
    let folded = analyze(&mut generic_analyzer(), word);
    if folded.len() > 1 {
        return phrase_query(f, &folded);
    }
    let token = folded.into_iter().next()?;
    let mut should: Vec<Box<dyn Query>> = vec![
        Box::new(TermQuery::new(Term::from_field_text(f.body, &token), IndexRecordOption::WithFreqs)),
        Box::new(BoostQuery::new(
            Box::new(TermQuery::new(Term::from_field_text(f.name, &token), IndexRecordOption::Basic)),
            2.0,
        )),
    ];
    for lang in DocLang::STEMMED.into_iter().filter(|_| mode.forms) {
        let (Some(field), Some(stemmed)) = (f.stem(lang), analyze(&mut stem_analyzer(lang), word).into_iter().next()) else {
            continue;
        };
        should.push(Box::new(BoostQuery::new(
            Box::new(TermQuery::new(Term::from_field_text(field, &stemmed), IndexRecordOption::WithFreqs)),
            0.7,
        )));
    }
    // `ab` off: the word may be inside a longer one ("contrat" → "sous-contrats").
    if let Some(longer) = mode.contains.0.get(&token).filter(|l| !l.is_empty()) {
        let terms = longer.iter().map(|t| Term::from_field_text(f.body, t));
        should.push(Box::new(BoostQuery::new(Box::new(TermSetQuery::new(terms)), 0.6)));
    }
    if mode.fuzzy && mode.forms {
        let distance = fuzzy_distance(&token);
        if distance > 0 {
            should.push(Box::new(BoostQuery::new(
                Box::new(FuzzyTermQuery::new(Term::from_field_text(f.body, &token), distance, true)),
                0.5,
            )));
        }
    }
    Some(Box::new(BooleanQuery::union(should)))
}

/// Paths starting with `prefix` (which ends with a separator): the range
/// from the prefix to the same text with its last character + 1.
fn prefix_query(f: &Fields, prefix: &str) -> Option<Box<dyn Query>> {
    let last = prefix.chars().next_back()?;
    let next = char::from_u32(last as u32 + 1)?;
    let upper = format!("{}{next}", &prefix[..prefix.len() - last.len_utf8()]);
    let lower = Bound::Included(Term::from_field_text(f.path, prefix));
    Some(Box::new(RangeQuery::new(lower, Bound::Excluded(Term::from_field_text(f.path, &upper)))))
}

fn phrase_query(f: &Fields, folded: &[String]) -> Option<Box<dyn Query>> {
    match folded {
        [] => None,
        [one] => Some(Box::new(TermQuery::new(Term::from_field_text(f.body, one), IndexRecordOption::WithFreqs))),
        many => Some(Box::new(PhraseQuery::new(many.iter().map(|t| Term::from_field_text(f.body, t)).collect()))),
    }
}

fn clause_query(f: &Fields, clause: &Clause, mode: WordMode) -> Option<Box<dyn Query>> {
    match clause {
        Clause::Word(w) => word_query(f, w, mode),
        Clause::Phrase(p) => phrase_query(f, &analyze(&mut generic_analyzer(), p)),
        Clause::Like(w) => word_query(f, w, WordMode { fuzzy: true, ..mode }),
        // The index narrows to files with every term; the distance is
        // checked on the text afterwards (`QueryLogic`).
        Clause::Near { terms, .. } => {
            let parts: Vec<Box<dyn Query>> = terms.iter().filter_map(|t| clause_query(f, t, mode)).collect();
            (!parts.is_empty()).then(|| Box::new(BooleanQuery::intersection(parts)) as Box<dyn Query>)
        }
        // Regular expressions and detectors are applied to the stored text afterwards.
        Clause::Regex(_) | Clause::Detector(_) => None,
    }
}

/// Boolean logic of the query on one text: every AND group has a clause that
/// matches, and no NOT clause matches. Checks what the index cannot (regex,
/// NEAR distance, LINES), and does all the work in live scan.
pub struct QueryLogic {
    groups: Vec<Vec<Matcher>>,
    excluded: Vec<Matcher>,
}

impl QueryLogic {
    pub fn new(parsed: &ParsedQuery, req: &SearchRequest) -> Result<Self> {
        let single = |clause: &Clause| req.matcher(&ParsedQuery { must: vec![vec![clause.clone()]], must_not: Vec::new(), lines: parsed.lines });
        let groups = parsed.must.iter().map(|g| g.iter().map(single).collect::<Result<Vec<_>>>()).collect::<Result<_>>()?;
        let excluded = parsed.must_not.iter().map(single).collect::<Result<_>>()?;
        Ok(Self { groups, excluded })
    }

    pub fn accepts(&self, text: &str, lang: DocLang) -> bool {
        self.groups.iter().all(|g| g.iter().any(|m| !m.find(text, lang).is_empty()))
            && !self.excluded.iter().any(|m| !m.find(text, lang).is_empty())
    }
}

pub(crate) fn build_query(
    f: &Fields,
    parsed: &ParsedQuery,
    req: &SearchRequest,
    names: Option<&NamePattern>,
    prefixes: &[String],
    contains: &Contains,
) -> Result<Box<dyn Query>> {
    let mode = WordMode { fuzzy: req.fuzzy, forms: !req.case_sensitive, contains };
    let mut clauses: Vec<(Occur, Box<dyn Query>)> = Vec::new();
    if !prefixes.is_empty() {
        let ranges: Vec<Box<dyn Query>> = prefixes.iter().filter_map(|p| prefix_query(f, p)).collect();
        clauses.push((Occur::Must, Box::new(BooleanQuery::union(ranges))));
    }
    if let Some(names) = names {
        clauses.push((Occur::Must, names.query(f.name_raw)?));
    }
    if !req.within_paths.is_empty() {
        let terms = req.within_paths.iter().map(|p| Term::from_field_text(f.path, p));
        clauses.push((Occur::Must, Box::new(TermSetQuery::new(terms))));
    }
    if !req.in_files.is_empty() {
        let terms = req.in_files.iter().map(|p| Term::from_field_text(f.file, p));
        clauses.push((Occur::Must, Box::new(TermSetQuery::new(terms))));
    }

    for group in &parsed.must {
        let alternatives: Vec<Box<dyn Query>> =
            group.iter().filter_map(|c| clause_query(f, c, mode)).collect();
        match alternatives.len() {
            0 => {}
            1 => clauses.extend(alternatives.into_iter().map(|q| (Occur::Must, q))),
            _ => clauses.push((Occur::Must, Box::new(BooleanQuery::union(alternatives)))),
        }
    }
    for clause in &parsed.must_not {
        // `NOT (a NEAR b)` excludes only files where they are close: checked on the text.
        // With `Aa`, `NOT Brouillon` must keep "brouillon": the text decides.
        if matches!(clause, Clause::Near { .. }) || req.case_sensitive {
            continue;
        }
        if let Some(q) = clause_query(f, clause, WordMode { fuzzy: false, ..mode }) {
            clauses.push((Occur::MustNot, q));
        }
    }

    // Filters.
    if !req.kinds.is_empty() {
        let terms = req.kinds.iter().map(|k| Term::from_field_text(f.kind, k));
        clauses.push((Occur::Must, Box::new(TermSetQuery::new(terms))));
    }
    if !req.langs.is_empty() {
        let terms = req.langs.iter().map(|l| Term::from_field_text(f.lang, l));
        clauses.push((Occur::Must, Box::new(TermSetQuery::new(terms))));
    }
    if req.min_size.is_some() || req.max_size.is_some() {
        let lower = req.min_size.map_or(Bound::Unbounded, |v| Bound::Included(Term::from_field_u64(f.size, v)));
        let upper = req.max_size.map_or(Bound::Unbounded, |v| Bound::Excluded(Term::from_field_u64(f.size, v)));
        clauses.push((Occur::Must, Box::new(RangeQuery::new(lower, upper))));
    }
    let date_field = match req.date_field {
        DateField::Modified => Some(f.modified),
        DateField::Created => Some(f.created),
        DateField::Accessed => None,
    };
    if let Some(field) = date_field.filter(|_| req.date_from.is_some() || req.date_to.is_some()) {
        let lower = req.date_from.map_or(Bound::Unbounded, |v| Bound::Included(Term::from_field_u64(field, v)));
        let upper = req.date_to.map_or(Bound::Unbounded, |v| Bound::Excluded(Term::from_field_u64(field, v)));
        clauses.push((Occur::Must, Box::new(RangeQuery::new(lower, upper))));
    }

    if !clauses.iter().any(|(occur, _)| *occur == Occur::Must) {
        clauses.push((Occur::Must, Box::new(AllQuery)));
    }
    Ok(Box::new(BooleanQuery::new(clauses)))
}

fn first_str(doc: &TantivyDocument, field: tantivy::schema::Field) -> String {
    doc.get_first(field).and_then(|v| v.as_str()).unwrap_or_default().to_owned()
}

fn first_u64(doc: &TantivyDocument, field: tantivy::schema::Field) -> u64 {
    doc.get_first(field).and_then(|v| v.as_u64()).unwrap_or_default()
}

/// What one site answers.
pub struct SiteAnswer {
    pub hits: Vec<Hit>,
    /// Matching documents (all of them, not only the hits returned).
    pub total: usize,
    pub detections: DetectionTotals,
    /// Counts per criterion, when asked (lot 6.3).
    pub facets: Option<Facets>,
}

/// Searches one site.
/// `prefixes`: only the documents under these folders (a search limited to a
/// folder, see scope.rs); empty = the whole site.
pub fn search_site(
    site_id: &str,
    index: &SiteIndex,
    parsed: &ParsedQuery,
    req: &SearchRequest,
    prefixes: &[String],
) -> Result<SiteAnswer> {
    let f = &index.fields;
    let searcher = index.reader.searcher();
    let matcher = req.matcher(parsed)?;
    // Tantivy answers the words; regular expressions filter its candidates.
    let names = req.names()?;
    let contains = Contains::expand(index, parsed, req)?;
    let query = build_query(f, &parsed.without_regexes(), req, names.as_ref(), prefixes, &contains)?;
    let limit = req.limit.unwrap_or(DEFAULT_LIMIT).max(1);
    let verify = req.verified(parsed);
    let logic = if verify { Some(QueryLogic::new(parsed, req)?) } else { None };
    // Last access, attributes, digest: the index gives the candidates, the
    // disk decides (last, only for what passed everything else).
    let disk = req.disk_check()?;
    let checked = verify || disk.is_some();
    let fetch = if checked { VERIFIED_CANDIDATES } else { limit };
    let (top, total) = searcher.search(&query, &(TopDocs::with_limit(fetch).order_by_score(), Count))?;

    let mut hits = Vec::with_capacity(top.len().min(limit));
    let mut dropped = 0;
    // Counts per criterion: from the index when it answers alone; otherwise
    // from the documents checked on their text, with the current filters.
    let mut facets = match (req.facets, checked) {
        (true, false) => Some(facet_counts(index, parsed, req, names.as_ref(), prefixes, &contains)?),
        (true, true) => Some(Facets::default()),
        (false, _) => None,
    };
    let date_field = if date_field_name(req) == "created" { f.created } else { f.modified };
    // Detectors (an audit), checked counts: every candidate is counted,
    // beyond the files shown.
    let counting = !req.detectors.is_empty() || (req.facets && checked);
    let mut totals = DetectionTotals::new();
    let mut accepted = 0;

    // One candidate, checked on its text (and the disk): its hit and date
    // when it is accepted.
    let examine = |score: f32, address: DocAddress| -> Result<Option<(Hit, u64)>> {
        let doc: TantivyDocument = searcher.doc(address)?;
        let lang_code = first_str(&doc, f.lang);
        let lang = DocLang::from_code(&lang_code).unwrap_or(DocLang::Und);
        let body = first_str(&doc, f.body);
        if logic.as_ref().is_some_and(|l| !l.accepts(&body, lang)) {
            return Ok(None);
        }
        let matches = if matcher.is_empty() { Vec::new() } else { matcher.find(&body, lang) };
        if !matcher.is_empty() && matches.is_empty() && !name_has_word(&matcher, &first_str(&doc, f.name)) {
            return Ok(None);
        }
        let path = first_str(&doc, f.path);
        if let Some(disk) = &disk {
            let (file, inner) = split_inner(&path);
            if !disk.accepts(Path::new(file), inner.is_some(), None) {
                return Ok(None);
            }
        }
        let exact_count = matches.iter().filter(|m| !m.fuzzy).count();
        // A file found only through typo tolerance goes after the exact ones.
        let score = if exact_count == 0 && !matches.is_empty() { score * APPROXIMATE_PENALTY } else { score };
        let hit = Hit {
            site_id: site_id.to_owned(),
            path,
            kind: first_str(&doc, f.kind),
            lang: lang_code,
            size_bytes: first_u64(&doc, f.size),
            modified: first_u64(&doc, f.modified),
            created: first_u64(&doc, f.created),
            detections: matcher.detections(&body),
            match_count: matches.len(),
            exact_count,
            score,
            snippets: snippets(&body, &matches, 2),
            inner_kind: None,
            meaning: None,
        }
        .with_inner_kind();
        Ok(Some((hit, first_u64(&doc, date_field))))
    };
    // Candidates are read and checked in parallel batches (a checked search
    // may read thousands of texts), then taken in score order.
    for batch in top.chunks(VERIFY_BATCH) {
        if hits.len() >= limit && !counting {
            break;
        }
        let examined: Vec<Result<Option<(Hit, u64)>>> = batch.par_iter().map(|&(score, address)| examine(score, address)).collect();
        for result in examined {
            let Some((hit, date)) = result? else {
                dropped += 1;
                continue;
            };
            accepted += 1;
            if let Some(facets) = facets.as_mut().filter(|_| checked) {
                facets.count_document(&hit.kind, &hit.lang, date);
            }
            add_detections(&mut totals, &hit.detections);
            if hits.len() < limit {
                hits.push(hit);
            }
        }
    }
    // With a text or disk check, only the examined candidates are known.
    let total = if checked { accepted } else { total.saturating_sub(dropped) };
    Ok(SiteAnswer { hits, total, detections: totals, facets })
}

/// Files can also match by name only ("contrat.pdf" with an empty body).
fn name_has_word(matcher: &Matcher, name: &str) -> bool {
    !matcher.find(name, DocLang::Und).is_empty()
}

/// Stored text of one document, by path.
pub fn stored_body(index: &SiteIndex, path: &str) -> Result<Option<(String, DocLang, String)>> {
    let f = &index.fields;
    let searcher = index.reader.searcher();
    let query = TermQuery::new(Term::from_field_text(f.path, path), IndexRecordOption::Basic);
    let top = searcher.search(&query, &TopDocs::with_limit(1).order_by_score())?;
    let Some((_, address)) = top.into_iter().next() else {
        return Ok(None);
    };
    let doc: TantivyDocument = searcher.doc(address)?;
    let lang = DocLang::from_code(&first_str(&doc, f.lang)).unwrap_or(DocLang::Und);
    Ok(Some((first_str(&doc, f.body), lang, first_str(&doc, f.kind))))
}
