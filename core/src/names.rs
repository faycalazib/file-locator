//! File-name criterion (goal.md §4bis, lot 5.1), next to the text query:
//!
//! - `facture` → names containing "facture";
//! - `*.pdf; facture-2024-??.xlsx` → wildcards (`*` any run, `?` one character),
//!   several patterns separated by `;` or `,`;
//! - `!*.tmp` → excludes the names matching this pattern;
//! - `/^inv-\d+/` → a regular expression.
//!
//! Names are compared without case or accents ([`fold`]): `resume*` finds
//! "Résumé.pdf". The index stores that folded name in `name_raw`, so the same
//! patterns become Tantivy `RegexQuery`s (which always match the whole term).

use regex::Regex;
use tantivy::query::{BooleanQuery, Occur, Query, RegexQuery};
use tantivy::schema::Field;

use crate::error::{CoreError, Result};
use crate::lang::fold;

#[derive(Debug)]
struct Pattern {
    /// Regex matching the **whole** folded name (no anchors: Tantivy style).
    source: String,
    compiled: Regex,
}

#[derive(Debug)]
pub struct NamePattern {
    include: Vec<Pattern>,
    exclude: Vec<Pattern>,
}

impl NamePattern {
    /// `None` for an empty criterion. An invalid regular expression is an
    /// `InvalidQuery` error (translated by the UI).
    pub fn parse(input: &str) -> Result<Option<Self>> {
        let mut include = Vec::new();
        let mut exclude = Vec::new();
        for item in split_items(input) {
            let (negated, item) = match item.strip_prefix('!') {
                Some(rest) => (true, rest.trim()),
                None => (false, item),
            };
            if item.is_empty() {
                continue;
            }
            let source = to_regex(item);
            let compiled = Regex::new(&format!("^(?:{source})$"))
                .map_err(|e| CoreError::InvalidQuery { message: e.to_string() })?;
            let pattern = Pattern { source, compiled };
            if negated {
                exclude.push(pattern);
            } else {
                include.push(pattern);
            }
        }
        if include.is_empty() && exclude.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self { include, exclude }))
    }

    /// Whether a file or folder name (not a path) matches.
    pub fn matches(&self, name: &str) -> bool {
        self.matches_any(&[name])
    }

    /// A document known under several names (an entry of an archive: its own
    /// name and the archive's). Like the index: one included name is enough,
    /// one excluded name is too many.
    pub fn matches_any(&self, names: &[&str]) -> bool {
        let folded: Vec<String> = names.iter().map(|n| fold(n)).collect();
        let any = |patterns: &[Pattern]| patterns.iter().any(|p| folded.iter().any(|n| p.compiled.is_match(n)));
        (self.include.is_empty() || any(&self.include)) && !any(&self.exclude)
    }

    /// The same criterion as an index query on the folded-name field.
    pub fn query(&self, name_raw: Field) -> Result<Box<dyn Query>> {
        let regex_query = |p: &Pattern| -> Result<Box<dyn Query>> {
            RegexQuery::from_pattern(&p.source, name_raw)
                .map(|q| Box::new(q) as Box<dyn Query>)
                .map_err(|e| CoreError::InvalidQuery { message: e.to_string() })
        };
        let mut clauses: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        if !self.include.is_empty() {
            let any = self.include.iter().map(regex_query).collect::<Result<Vec<_>>>()?;
            clauses.push((Occur::Must, Box::new(BooleanQuery::union(any))));
        } else {
            clauses.push((Occur::Must, Box::new(tantivy::query::AllQuery)));
        }
        for pattern in &self.exclude {
            clauses.push((Occur::MustNot, regex_query(pattern)?));
        }
        Ok(Box::new(BooleanQuery::new(clauses)))
    }
}

/// `a; b, c` → items. A `/regex/` item is kept whole, even with `;` or `,` inside.
fn split_items(input: &str) -> Vec<&str> {
    let trimmed = input.trim();
    let bare = trimmed.strip_prefix('!').unwrap_or(trimmed).trim_start();
    if bare.len() > 2 && bare.starts_with('/') && bare.ends_with('/') {
        return vec![trimmed];
    }
    trimmed.split([';', ',']).map(str::trim).filter(|s| !s.is_empty()).collect()
}

/// One item → a regex matching the whole folded name.
fn to_regex(item: &str) -> String {
    if item.len() > 2 && item.starts_with('/') && item.ends_with('/') {
        let inner = &item[1..item.len() - 1];
        // Tantivy regexes always cover the whole term: anchors become the
        // absence of the surrounding `.*`.
        let (start, inner) = match inner.strip_prefix('^') {
            Some(rest) => ("", rest),
            None => (".*", inner),
        };
        let (inner, end) = match inner.strip_suffix('$') {
            Some(rest) if !rest.ends_with('\\') => (rest, ""),
            _ => (inner, ".*"),
        };
        return format!("{start}(?:{inner}){end}");
    }
    let folded = fold(item);
    let has_wildcard = folded.contains(['*', '?']);
    let mut source = String::new();
    for c in folded.chars() {
        match c {
            '*' => source.push_str(".*"),
            '?' => source.push('.'),
            other => source.push_str(&regex::escape(&other.to_string())),
        }
    }
    // A plain word finds the names that contain it.
    if has_wildcard {
        source
    } else {
        format!(".*{source}.*")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(input: &str) -> NamePattern {
        NamePattern::parse(input).unwrap().expect("not empty")
    }

    #[test]
    fn wildcards_words_and_exclusions() {
        let p = pattern("*.pdf; facture-2024-??.xlsx");
        assert!(p.matches("Contrat.PDF"));
        assert!(p.matches("facture-2024-03.xlsx"));
        assert!(!p.matches("facture-2024-3.xlsx"));
        assert!(!p.matches("notes.txt"));

        let word = pattern("resume");
        assert!(word.matches("Mon Résumé 2024.docx"), "no accents, no case");

        let except = pattern("*.txt, !brouillon*");
        assert!(except.matches("notes.txt"));
        assert!(!except.matches("Brouillon-v2.txt"));

        let only_exclude = pattern("!*.tmp");
        assert!(only_exclude.matches("a.pdf"));
        assert!(!only_exclude.matches("x.tmp"));

        assert!(pattern("عقد").matches("عَقْد_إيجار.pdf"));
        assert!(NamePattern::parse("  ; ").unwrap().is_none());

        // Archive entry: its own name or the archive's; any excluded name wins.
        assert!(pattern("*.rar").matches_any(&["devis.txt", "chantier.rar"]));
        assert!(!pattern("!*.docx").matches_any(&["annexe.docx", "dossier.zip"]));
    }

    #[test]
    fn regular_expressions() {
        let p = pattern(r"/^inv-\d{4}/");
        assert!(p.matches("INV-2024-117.pdf"));
        assert!(!p.matches("old-inv-2024.pdf"));
        let end = pattern(r"/\.(docx|pdf)$/");
        assert!(end.matches("a.docx"));
        assert!(!end.matches("a.docx.bak"));
        // A regex may contain `;` or `,`.
        assert!(pattern("/a{1,2}b/").matches("xaab"));
        assert!(NamePattern::parse("/[a/").is_err());
    }
}
