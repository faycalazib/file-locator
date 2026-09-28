//! Keyword report (lot 6.4): for each file found, how many times each term
//! of the query occurs, and the totals per term.

use serde::{Deserialize, Serialize};

use crate::query::{Clause, ParsedQuery};

/// A document of the report: its site (to find its stored text) and path.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocRef {
    pub site_id: String,
    pub path: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordReport {
    /// The terms as the user wrote them (`contrat`, `"clause pénale"`, `/INV-\d+/`).
    pub terms: Vec<String>,
    pub rows: Vec<KeywordRow>,
    pub totals: Vec<TermTotal>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordRow {
    pub site_id: String,
    pub path: String,
    /// Occurrences of each term, in the order of `terms`.
    pub counts: Vec<usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TermTotal {
    pub files: usize,
    pub matches: usize,
}

fn label(clause: &Clause) -> Option<String> {
    Some(match clause {
        Clause::Word(w) => w.clone(),
        Clause::Phrase(p) => format!("\"{p}\""),
        Clause::Regex(r) => format!("/{r}/"),
        Clause::Like(w) => format!("LIKE {w}"),
        Clause::Near { terms, chars } => {
            let parts: Vec<String> = terms.iter().filter_map(label).collect();
            format!("{} (NEAR:{chars})", parts.join(" NEAR "))
        }
        // Detected data have their own columns (lot 6.2).
        Clause::Detector(_) => return None,
    })
}

/// The positive terms of a query, each once, with their label.
pub fn terms(parsed: &ParsedQuery) -> Vec<(String, Clause)> {
    let mut out: Vec<(String, Clause)> = Vec::new();
    for clause in parsed.positive_clauses() {
        if let Some(name) = label(clause) {
            if !out.iter().any(|(n, _)| *n == name) {
                out.push((name, clause.clone()));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::parse;

    #[test]
    fn labels_of_the_terms() {
        let names: Vec<String> = terms(&parse(r#"contrat OR "clause pénale" /INV-\d+/ LIKE paiement contrat NOT brouillon"#))
            // `contrat` twice: once; `brouillon` is excluded, not a term.
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert_eq!(names, ["contrat", "\"clause pénale\"", r"/INV-\d+/", "LIKE paiement"]);
    }
}
