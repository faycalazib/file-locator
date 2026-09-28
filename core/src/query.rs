//! User query language (goal.md §3):
//!   `contrat`                 → word
//!   `"clause de confidentialité"` / `«…»` → exact phrase
//!   `a OR b`                  → either (also `|`)
//!   `a AND b`, `a b`          → both (default)
//!   `NOT a`, `-a`             → excluded
//!   `/INV-\d{4}/`            → regular expression (always required)
//!   `a NEAR b`, `a NEAR:20 b` → both within 100 (or 20) characters (lot 5.5)
//!   `LIKE mot`                → this word with typo tolerance, whatever the option
//!   `LINES:3-5 …`, `LINES:10+ …` → only in those lines of the document
//! The syntax of NEAR, LIKE and LINES is FileLocator Pro's.
//! Operators are only recognized in UPPER CASE, so the English word "or"
//! stays a normal search word. Parentheses are accepted and ignored
//! (`LINES:3-5 (tower AND london)`).

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Clause {
    Word(String),
    Phrase(String),
    /// Regular expression, applied to the text of each candidate document.
    Regex(String),
    /// Every term (word or phrase) within `chars` characters of the others.
    Near { terms: Vec<Clause>, chars: usize },
    /// A word with typo tolerance, even when the option is off.
    Like(String),
    /// Validated data (IBAN, card…, lot 6.2), checked on the text.
    Detector(crate::detect::Detector),
}

/// Default distance of NEAR, in characters (FileLocator Pro's).
pub const NEAR_CHARS: usize = 100;

/// `must` is a conjunction of groups; each group is a disjunction of clauses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParsedQuery {
    pub must: Vec<Vec<Clause>>,
    pub must_not: Vec<Clause>,
    /// `LINES:a-b`: 1-based inclusive range of lines to search (`b` =
    /// `usize::MAX` for `LINES:a+`).
    pub lines: Option<(usize, usize)>,
}

impl ParsedQuery {
    pub fn is_empty(&self) -> bool {
        self.must.is_empty() && self.must_not.is_empty()
    }

    /// The index can only narrow the candidates: each one must then be
    /// checked on its text (regex, proximity, line range).
    pub fn needs_verification(&self) -> bool {
        self.lines.is_some()
            || self.must.iter().flatten().chain(&self.must_not).any(|c| matches!(c, Clause::Regex(_) | Clause::Near { .. } | Clause::Detector(_)))
    }

    /// Every positive clause, for highlighting.
    pub fn positive_clauses(&self) -> impl Iterator<Item = &Clause> {
        self.must.iter().flatten()
    }

    /// Regular expressions of the query (they are always required).
    pub fn regexes(&self) -> impl Iterator<Item = &str> {
        self.must.iter().flatten().filter_map(|c| match c {
            Clause::Regex(r) => Some(r.as_str()),
            _ => None,
        })
    }

    /// The query without its regular expressions (what Tantivy can answer).
    pub fn without_regexes(&self) -> ParsedQuery {
        ParsedQuery {
            must: self
                .must
                .iter()
                .map(|g| g.iter().filter(|c| !matches!(c, Clause::Regex(_) | Clause::Detector(_))).cloned().collect::<Vec<_>>())
                .filter(|g| !g.is_empty())
                .collect(),
            must_not: self.must_not.clone(),
            lines: self.lines,
        }
    }

    /// The whole input as one regular expression (the `.*` option).
    pub fn whole_regex(input: &str) -> ParsedQuery {
        let pattern = input.trim();
        ParsedQuery {
            must: if pattern.is_empty() { Vec::new() } else { vec![vec![Clause::Regex(pattern.to_owned())]] },
            must_not: Vec::new(),
            lines: None,
        }
    }
}

#[derive(Debug, PartialEq)]
enum Token {
    Clause(Clause),
    Or,
    And,
    Not,
    /// `NEAR` / `NEAR:n` (characters).
    Near(usize),
    Like,
    /// `LINES:a-b` / `LINES:a+` / `LINES:a`.
    Lines(usize, usize),
}

/// `3-5` → (3, 5), `10+` → (10, MAX), `7` → (7, 7). Lines count from 1.
fn line_range(spec: &str) -> Option<(usize, usize)> {
    let range = if let Some(from) = spec.strip_suffix('+') {
        (from.parse().ok()?, usize::MAX)
    } else if let Some((a, b)) = spec.split_once('-') {
        (a.parse().ok()?, b.parse().ok()?)
    } else {
        let n = spec.parse().ok()?;
        (n, n)
    };
    (range.0 >= 1 && range.0 <= range.1).then_some(range)
}

fn lex(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&c) = chars.peek() {
        // Parentheses group nothing here: `LINES:3-5 (a AND b)` = `LINES:3-5 a AND b`.
        if c.is_whitespace() || c == '(' || c == ')' {
            chars.next();
            continue;
        }
        let closing = match c {
            '"' => Some('"'),
            '«' => Some('»'),
            '“' => Some('”'),
            _ => None,
        };
        if let Some(close) = closing {
            chars.next();
            let phrase: String = chars.by_ref().take_while(|&ch| ch != close).collect();
            let phrase = phrase.trim().to_owned();
            if !phrase.is_empty() {
                tokens.push(Token::Clause(Clause::Phrase(phrase)));
            }
            continue;
        }
        if c == '/' {
            // `/…/`: everything up to the closing slash (escaped slashes kept).
            let mut pattern = String::new();
            let mut closed = false;
            chars.next();
            while let Some(ch) = chars.next() {
                if ch == '\\' {
                    pattern.push(ch);
                    if let Some(next) = chars.next() {
                        pattern.push(next);
                    }
                    continue;
                }
                if ch == '/' {
                    closed = true;
                    break;
                }
                pattern.push(ch);
            }
            if closed && !pattern.is_empty() {
                tokens.push(Token::Clause(Clause::Regex(pattern)));
            } else if !pattern.is_empty() {
                tokens.push(Token::Clause(Clause::Word(pattern)));
            }
            continue;
        }
        if c == '|' {
            chars.next();
            tokens.push(Token::Or);
            continue;
        }
        let mut word = String::new();
        while let Some(&ch) = chars.peek() {
            if ch.is_whitespace() || matches!(ch, '"' | '«' | '“' | '|' | '(' | ')') {
                break;
            }
            word.push(ch);
            chars.next();
        }
        match word.as_str() {
            "OR" => tokens.push(Token::Or),
            "AND" => tokens.push(Token::And),
            "NOT" => tokens.push(Token::Not),
            "NEAR" => tokens.push(Token::Near(NEAR_CHARS)),
            "LIKE" => tokens.push(Token::Like),
            w if w.starts_with("NEAR:") && w[5..].parse::<usize>().is_ok() => {
                tokens.push(Token::Near(w[5..].parse().unwrap_or(NEAR_CHARS)));
            }
            w if w.starts_with("LINES:") && line_range(&w[6..]).is_some() => {
                if let Some((a, b)) = line_range(&w[6..]) {
                    tokens.push(Token::Lines(a, b));
                }
            }
            _ => {
                if let Some(rest) = word.strip_prefix('-').filter(|r| !r.is_empty()) {
                    tokens.push(Token::Not);
                    tokens.push(Token::Clause(Clause::Word(rest.to_owned())));
                } else if !word.is_empty() {
                    tokens.push(Token::Clause(Clause::Word(word)));
                }
            }
        }
    }
    tokens
}

/// Words, phrases and fuzzy words can be near each other (and a NEAR chain).
fn is_near_term(clause: &Clause) -> bool {
    matches!(clause, Clause::Word(_) | Clause::Phrase(_) | Clause::Like(_) | Clause::Near { .. })
}

pub fn parse(input: &str) -> ParsedQuery {
    let mut query = ParsedQuery::default();
    let mut negate = false;
    let mut join_or = false;
    let mut near: Option<usize> = None;
    let mut like = false;
    // Where the previous clause went: NEAR joins it (`NOT a NEAR b`).
    let mut last_negated = false;

    for token in lex(input) {
        match token {
            Token::Or => join_or = true,
            Token::And => join_or = false,
            Token::Not => negate = true,
            Token::Near(chars) => near = Some(chars),
            Token::Like => like = true,
            Token::Lines(a, b) => query.lines = Some((a, b)),
            Token::Clause(clause) => {
                let clause = match clause {
                    Clause::Word(w) if std::mem::take(&mut like) => Clause::Like(w),
                    other => other,
                };
                // `a NEAR b`: the previous clause and this one become one.
                if let Some(chars) = near.take() {
                    let target = if last_negated { query.must_not.last_mut() } else { query.must.last_mut().and_then(|g| g.last_mut()) };
                    if let Some(previous) = target.filter(|p| is_near_term(p) && is_near_term(&clause)) {
                        *previous = match std::mem::replace(previous, Clause::Word(String::new())) {
                            Clause::Near { mut terms, chars: d } => {
                                terms.push(clause);
                                Clause::Near { terms, chars: d.min(chars) }
                            }
                            single => Clause::Near { terms: vec![single, clause], chars },
                        };
                        negate = false;
                        join_or = false;
                        continue;
                    }
                }
                last_negated = negate;
                if negate {
                    query.must_not.push(clause);
                } else if join_or && !query.must.is_empty() {
                    if let Some(group) = query.must.last_mut() {
                        group.push(clause);
                    }
                } else {
                    query.must.push(vec![clause]);
                }
                negate = false;
                join_or = false;
            }
        }
    }
    query
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(s: &str) -> Clause {
        Clause::Word(s.into())
    }

    #[test]
    fn or_groups_and_default_and() {
        let q = parse("contrat OR contrato OR عقد");
        assert_eq!(q.must, vec![vec![w("contrat"), w("contrato"), w("عقد")]]);
        let q = parse("facture impayée");
        assert_eq!(q.must, vec![vec![w("facture")], vec![w("impayée")]]);
    }

    #[test]
    fn phrases_and_negations() {
        let q = parse("« clause de confidentialité » résiliation NOT avenant -brouillon");
        assert_eq!(q.must, vec![vec![Clause::Phrase("clause de confidentialité".into())], vec![w("résiliation")]]);
        assert_eq!(q.must_not, vec![w("avenant"), w("brouillon")]);
    }

    #[test]
    fn regex_clauses() {
        let q = parse(r"facture /INV-\d{4}\/x/ -brouillon");
        assert_eq!(q.must, vec![vec![w("facture")], vec![Clause::Regex(r"INV-\d{4}\/x".into())]]);
        assert_eq!(q.regexes().collect::<Vec<_>>(), [r"INV-\d{4}\/x"]);
        assert_eq!(q.without_regexes().must, vec![vec![w("facture")]]);
    }

    #[test]
    fn near_like_and_lines() {
        let q = parse("contrat NEAR résiliation facture NEAR:20 \"en attente\" NEAR retard");
        assert_eq!(
            q.must,
            vec![
                vec![Clause::Near { terms: vec![w("contrat"), w("résiliation")], chars: NEAR_CHARS }],
                vec![Clause::Near { terms: vec![w("facture"), Clause::Phrase("en attente".into()), w("retard")], chars: 20 }],
            ]
        );
        let q = parse("LIKE necessary LINES:3-5 (tower AND london)");
        assert_eq!(q.must, vec![vec![Clause::Like("necessary".into())], vec![w("tower")], vec![w("london")]]);
        assert_eq!(q.lines, Some((3, 5)));
        assert_eq!(parse("LINES:10+ x").lines, Some((10, usize::MAX)));
        assert!(q.needs_verification());
        // Lower case and malformed forms stay words.
        assert_eq!(parse("near LINES:0-2").must, vec![vec![w("near")], vec![w("LINES:0-2")]]);
        assert!(!parse("a b").needs_verification());
        let q = parse("b NOT a NEAR b");
        assert_eq!(q.must, vec![vec![w("b")]]);
        assert_eq!(q.must_not, vec![Clause::Near { terms: vec![w("a"), w("b")], chars: NEAR_CHARS }]);
    }

    #[test]
    fn lowercase_or_is_a_word() {
        let q = parse("this or that");
        assert_eq!(q.must.len(), 3);
    }
}
