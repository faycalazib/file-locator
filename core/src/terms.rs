//! Term lists (lot 7.4): search criteria read from a file, one term per line
//! (the first column of a CSV). Each term becomes a clause of the query:
//! `/…/` a regular expression, several words an exact phrase, otherwise a
//! word — the same rules as the search box. The list is one more AND group
//! of the query ("at least one of them"), or one group per term ("all").

use std::path::Path;

use serde::Serialize;

use crate::error::{CoreError, Result};
use crate::kind::FileKind;
use crate::query::Clause;

/// More than this is not a list of terms any more.
pub const MAX_TERMS: usize = 5000;
/// Files larger than this are refused.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TermList {
    pub terms: Vec<String>,
    /// Lines left out: comments, repeated terms, beyond the limit.
    pub skipped: usize,
}

/// The terms of a text: one per line; for a CSV / TSV, the first column
/// (quotes removed); `#` comments, empty lines and repeats left out.
pub fn parse_list(text: &str, csv: bool) -> TermList {
    let mut list = TermList::default();
    let mut seen = std::collections::HashSet::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            list.skipped += 1;
            continue;
        }
        let term = if csv { first_field(line) } else { line.to_owned() };
        let term = term.trim().to_owned();
        if term.is_empty() || !seen.insert(term.to_lowercase()) || list.terms.len() >= MAX_TERMS {
            list.skipped += 1;
            continue;
        }
        list.terms.push(term);
    }
    list
}

/// First field of a CSV line (`;`, `,` or tab), without its quotes.
fn first_field(line: &str) -> String {
    if let Some(rest) = line.strip_prefix('"') {
        // "a ""quoted"" name";… → a "quoted" name
        let mut out = String::new();
        let mut chars = rest.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    out.push('"');
                } else {
                    break;
                }
            } else {
                out.push(c);
            }
        }
        return out;
    }
    line.split([';', ',', '\t']).next().unwrap_or_default().to_owned()
}

/// Reads a list file (encoding detected like any text file).
pub fn read_list(path: &Path) -> Result<TermList> {
    let unreadable = || CoreError::Storage { message: format!("cannot read {}", path.display()) };
    if std::fs::metadata(path).map_err(|_| unreadable())?.len() > MAX_BYTES {
        return Err(unreadable());
    }
    let bytes = std::fs::read(path).map_err(|_| unreadable())?;
    let text = crate::extract::extract_bytes(&bytes, FileKind::Text, "list.txt").map_err(|_| unreadable())?;
    let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    Ok(parse_list(&text, matches!(ext.as_str(), "csv" | "tsv")))
}

/// The clause of one term.
pub fn clause(term: &str) -> Clause {
    let term = term.trim();
    if term.len() > 2 && term.starts_with('/') && term.ends_with('/') {
        Clause::Regex(term[1..term.len() - 1].to_owned())
    } else if term.contains(char::is_whitespace) {
        Clause::Phrase(term.to_owned())
    } else {
        Clause::Word(term.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_columns_comments_repeats() {
        let list = parse_list("\u{feff}Dupont SARL\n# fournisseurs 2024\n\n  García Hermanos  \n/FR\\d{11}/\ndupont sarl\n", false);
        assert_eq!(list.terms, ["Dupont SARL", "García Hermanos", "/FR\\d{11}/"]);
        assert_eq!(list.skipped, 2, "the comment and the repeat");
        let csv = parse_list("\"Dupont \"\"père\"\" SARL\";Lyon\nBâti-Sud,Toulouse\nRoux\tNîmes\n", true);
        assert_eq!(csv.terms, ["Dupont \"père\" SARL", "Bâti-Sud", "Roux"]);
        let many: String = (0..MAX_TERMS + 3).map(|i| format!("t{i}\n")).collect();
        let capped = parse_list(&many, false);
        assert_eq!((capped.terms.len(), capped.skipped), (MAX_TERMS, 3));
    }

    #[test]
    fn a_term_is_a_word_a_phrase_or_a_regex() {
        assert_eq!(clause("contrat"), Clause::Word("contrat".into()));
        assert_eq!(clause("Dupont SARL"), Clause::Phrase("Dupont SARL".into()));
        assert_eq!(clause("/INV-\\d+/"), Clause::Regex("INV-\\d+".into()));
        assert_eq!(clause("/"), Clause::Word("/".into()));
    }
}
