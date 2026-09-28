//! Counts per criterion (lot 6.3): how many documents found per file kind,
//! language, year and site, over everything found (not only the documents
//! returned). Each criterion is counted without its own filter: with "PDF"
//! chosen, the other kinds still show what widening would bring.

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use tantivy::collector::{Collector, Count, SegmentCollector};
use tantivy::columnar::{Column, StrColumn};
use tantivy::{DocId, Score, SegmentReader};

use crate::error::Result;
use crate::index::SiteIndex;
use crate::names::NamePattern;
use crate::query::ParsedQuery;
use crate::search::{build_query, Contains, DateField, SearchRequest};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Facets {
    pub kinds: BTreeMap<String, usize>,
    pub langs: BTreeMap<String, usize>,
    /// `"2026"` → documents.
    pub years: BTreeMap<String, usize>,
    /// Site id → documents (sites not searched included).
    pub sites: BTreeMap<String, usize>,
}

fn add(into: &mut BTreeMap<String, usize>, from: BTreeMap<String, usize>) {
    for (key, n) in from {
        *into.entry(key).or_insert(0) += n;
    }
}

impl Facets {
    pub fn merge(&mut self, other: Facets) {
        add(&mut self.kinds, other.kinds);
        add(&mut self.langs, other.langs);
        add(&mut self.years, other.years);
        add(&mut self.sites, other.sites);
    }

    /// One document examined on its text (queries the index cannot answer
    /// alone): counted with the current filters.
    pub fn count_document(&mut self, kind: &str, lang: &str, date: u64) {
        *self.kinds.entry(kind.to_owned()).or_insert(0) += 1;
        *self.langs.entry(lang.to_owned()).or_insert(0) += 1;
        *self.years.entry(year_of(date).to_string()).or_insert(0) += 1;
    }
}

/// Calendar year (UTC) of a Unix time in seconds.
pub fn year_of(secs: u64) -> i64 {
    // Days since 1970-01-01 → civil date (H. Hinnant's algorithm).
    let days = (secs / 86_400) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    yoe + era * 400 + i64::from(month <= 2)
}

/// The date the years are counted on: the one the date filter uses.
pub fn date_field_name(req: &SearchRequest) -> &'static str {
    if req.date_field == DateField::Created {
        "created"
    } else {
        "modified"
    }
}

#[derive(Clone, Copy)]
enum Facet {
    /// A raw text fast field (`kind`, `lang`).
    Text(&'static str),
    /// The year of a date fast field (`modified`, `created`).
    Year(&'static str),
}

struct FacetCollector(Facet);

enum SegmentCounts {
    Text(Option<StrColumn>, HashMap<u64, usize>),
    Year(Column<u64>, HashMap<i64, usize>),
}

impl Collector for FacetCollector {
    type Fruit = BTreeMap<String, usize>;
    type Child = SegmentCounts;

    fn for_segment(&self, _segment: u32, reader: &SegmentReader) -> tantivy::Result<Self::Child> {
        Ok(match self.0 {
            Facet::Text(field) => SegmentCounts::Text(reader.fast_fields().str(field)?, HashMap::new()),
            Facet::Year(field) => SegmentCounts::Year(reader.fast_fields().u64(field)?, HashMap::new()),
        })
    }

    fn requires_scoring(&self) -> bool {
        false
    }

    fn merge_fruits(&self, fruits: Vec<Self::Fruit>) -> tantivy::Result<Self::Fruit> {
        let mut all = BTreeMap::new();
        for fruit in fruits {
            add(&mut all, fruit);
        }
        Ok(all)
    }
}

impl SegmentCollector for SegmentCounts {
    type Fruit = BTreeMap<String, usize>;

    fn collect(&mut self, doc: DocId, _score: Score) {
        match self {
            SegmentCounts::Text(Some(column), counts) => {
                for ord in column.term_ords(doc) {
                    *counts.entry(ord).or_insert(0) += 1;
                }
            }
            SegmentCounts::Text(None, _) => {}
            SegmentCounts::Year(column, counts) => {
                if let Some(secs) = column.first(doc) {
                    *counts.entry(year_of(secs)).or_insert(0) += 1;
                }
            }
        }
    }

    fn harvest(self) -> Self::Fruit {
        match self {
            SegmentCounts::Text(Some(column), counts) => {
                let mut out = BTreeMap::new();
                let mut text = String::new();
                for (ord, n) in counts {
                    text.clear();
                    if column.ord_to_str(ord, &mut text).unwrap_or(false) {
                        *out.entry(text.clone()).or_insert(0) += n;
                    }
                }
                out
            }
            SegmentCounts::Text(None, _) => BTreeMap::new(),
            SegmentCounts::Year(_, counts) => counts.into_iter().map(|(year, n)| (year.to_string(), n)).collect(),
        }
    }
}

/// Kinds, languages and years of one site, for a query the index answers
/// alone. Each is counted with the other filters, without its own.
pub(crate) fn facet_counts(
    index: &SiteIndex,
    parsed: &ParsedQuery,
    req: &SearchRequest,
    names: Option<&NamePattern>,
    prefixes: &[String],
    contains: &Contains,
) -> Result<Facets> {
    let f = &index.fields;
    let searcher = index.reader.searcher();
    let base = parsed.without_regexes();
    let run = |request: &SearchRequest, facet: Facet| -> Result<BTreeMap<String, usize>> {
        let query = build_query(f, &base, request, names, prefixes, contains)?;
        Ok(searcher.search(&query, &FacetCollector(facet))?)
    };
    Ok(Facets {
        kinds: run(&SearchRequest { kinds: Vec::new(), ..req.clone() }, Facet::Text("kind"))?,
        langs: run(&SearchRequest { langs: Vec::new(), ..req.clone() }, Facet::Text("lang"))?,
        years: run(&SearchRequest { date_from: None, date_to: None, ..req.clone() }, Facet::Year(date_field_name(req)))?,
        sites: BTreeMap::new(),
    })
}

/// Documents of a site that is not searched (for its count in the rail).
pub fn count_site(index: &SiteIndex, parsed: &ParsedQuery, req: &SearchRequest, prefixes: &[String]) -> Result<usize> {
    let names = req.names()?;
    let contains = Contains::expand(index, parsed, req)?;
    let query = build_query(&index.fields, &parsed.without_regexes(), req, names.as_ref(), prefixes, &contains)?;
    Ok(index.reader.searcher().search(&query, &Count)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn years_from_unix_time() {
        assert_eq!(year_of(0), 1970);
        assert_eq!(year_of(951_782_400), 2000, "2000-02-29");
        assert_eq!(year_of(1_704_067_199), 2023, "2023-12-31 23:59:59");
        assert_eq!(year_of(1_704_067_200), 2024, "2024-01-01 00:00:00");
        assert_eq!(year_of(1_790_000_000), 2026);
    }
}
