//! What prospector-cli prints: text for people, JSON / JSON Lines / CSV for
//! scripts. Matches are marked `[like this]` in text; the other formats
//! give plain snippets (no marks).

use std::io::Write;

use clap::ValueEnum;
use prospector_core::{Hit, SiteGroup, SiteRecord};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Readable text (default).
    Text,
    /// One JSON document.
    Json,
    /// One JSON object per line, written as results come.
    Jsonl,
    /// CSV with a header line (UTF-8, comma).
    Csv,
}

/// One result, as scripts get it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HitOut {
    pub path: String,
    pub site: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_kind: Option<String>,
    pub lang: String,
    pub size: u64,
    /// ISO 8601, UTC.
    pub modified: String,
    pub created: String,
    pub matches: usize,
    /// Matches found without typo tolerance.
    pub exact: usize,
    pub score: f32,
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub detections: std::collections::BTreeMap<String, usize>,
    pub snippets: Vec<SnippetOut>,
    /// Found by meaning (`--meaning`): its closest passage's similarity, and
    /// whether no word of the search is in it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meaning: Option<MeaningOut>,
}

#[derive(Serialize)]
pub struct MeaningOut {
    pub score: f32,
    pub only: bool,
}

#[derive(Serialize)]
pub struct SnippetOut {
    pub line: usize,
    pub text: String,
}

/// The UI marks exact matches ⟦…⟧ and approximate ones ⟪…⟫.
fn marks(text: &str, open: &str, close: &str) -> String {
    text.replace(['⟦', '⟪'], open).replace(['⟧', '⟫'], close)
}

pub fn hit_out(hit: &Hit, site_name: &str) -> HitOut {
    HitOut {
        path: hit.path.clone(),
        site: site_name.to_owned(),
        kind: hit.kind.clone(),
        inner_kind: hit.inner_kind.clone(),
        lang: hit.lang.clone(),
        size: hit.size_bytes,
        modified: iso_date(hit.modified),
        created: iso_date(hit.created),
        matches: hit.match_count,
        exact: hit.exact_count,
        score: hit.score,
        detections: hit.detections.clone(),
        meaning: hit.meaning.as_ref().map(|m| MeaningOut { score: m.score, only: m.only }),
        snippets: hit.snippets.iter().map(|s| SnippetOut { line: s.line, text: marks(&s.text, "", "") }).collect(),
    }
}

/// Unix seconds → `2024-03-15T10:30:20Z` (days to civil date, H. Hinnant).
pub fn iso_date(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rest = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rest / 3600, rest % 3600 / 60, rest % 60)
}

/// `2024-03-15` (UTC midnight) → Unix seconds.
pub fn parse_date(text: &str) -> Option<u64> {
    let mut parts = text.trim().splitn(3, '-');
    let (y, m, d): (i64, i64, i64) = (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    u64::try_from((era * 146_097 + doe - 719_468) * 86_400).ok()
}

/// `500`, `10KB`, `2.5MB`, `1GB` → bytes.
pub fn parse_size(text: &str) -> Result<u64, String> {
    let t = text.trim().to_ascii_uppercase();
    let (number, unit) = match t.find(|c: char| c.is_ascii_alphabetic()) {
        Some(i) => t.split_at(i),
        None => (t.as_str(), ""),
    };
    let factor: f64 = match unit.trim() {
        "" | "B" => 1.0,
        "KB" | "K" => 1024.0,
        "MB" | "M" => 1024.0 * 1024.0,
        "GB" | "G" => 1024.0 * 1024.0 * 1024.0,
        _ => return Err(format!("unknown size unit in {text:?} (B, KB, MB, GB)")),
    };
    let value: f64 = number.trim().parse().map_err(|_| format!("not a size: {text:?}"))?;
    Ok((value * factor).round() as u64)
}

/// Writes results one after the other (text, JSON Lines, CSV) or gathers
/// them for one JSON document.
pub struct Printer<W: Write> {
    format: Format,
    out: W,
    gathered: Vec<HitOut>,
    header: bool,
}

const CSV_HEADER: &str = "path,site,kind,innerKind,lang,size,modified,created,matches,exact,score,meaning,snippet";

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) { format!("\"{}\"", value.replace('"', "\"\"")) } else { value.to_owned() }
}

impl<W: Write> Printer<W> {
    pub fn new(format: Format, out: W) -> Self {
        Self { format, out, gathered: Vec::new(), header: false }
    }

    pub fn hit(&mut self, hit: &Hit, site_name: &str) -> std::io::Result<()> {
        match self.format {
            Format::Text => {
                let detail = match (&hit.meaning, hit.match_count) {
                    (Some(m), _) if m.only => format!("{} · by meaning", hit.kind),
                    (Some(_), n) => format!("{} · {n} matches · also by meaning", hit.kind),
                    (None, 0) => hit.kind.clone(),
                    (None, n) => format!("{} · {n} matches", hit.kind),
                };
                writeln!(self.out, "{}  ({detail})", hit.path)?;
                for s in &hit.snippets {
                    writeln!(self.out, "  {:>5}: {}", s.line, marks(&s.text, "[", "]").trim())?;
                }
            }
            Format::Jsonl => {
                serde_json::to_writer(&mut self.out, &hit_out(hit, site_name))?;
                writeln!(self.out)?;
            }
            Format::Csv => {
                if !self.header {
                    writeln!(self.out, "{CSV_HEADER}")?;
                    self.header = true;
                }
                let o = hit_out(hit, site_name);
                let snippet = o.snippets.first().map(|s| s.text.trim().to_owned()).unwrap_or_default();
                let fields = [
                    o.path,
                    o.site,
                    o.kind,
                    o.inner_kind.unwrap_or_default(),
                    o.lang,
                    o.size.to_string(),
                    o.modified,
                    o.created,
                    o.matches.to_string(),
                    o.exact.to_string(),
                    format!("{:.3}", o.score),
                    o.meaning.as_ref().map(|m| format!("{:.3}{}", m.score, if m.only { " only" } else { "" })).unwrap_or_default(),
                    snippet,
                ];
                writeln!(self.out, "{}", fields.iter().map(|f| csv_field(f)).collect::<Vec<_>>().join(","))?;
            }
            Format::Json => self.gathered.push(hit_out(hit, site_name)),
        }
        self.out.flush()
    }

    /// Ends the output; `summary` goes into the JSON document.
    pub fn finish(mut self, summary: serde_json::Value) -> std::io::Result<()> {
        match self.format {
            Format::Json => {
                let mut document = summary;
                document["hits"] = serde_json::to_value(&self.gathered)?;
                serde_json::to_writer_pretty(&mut self.out, &document)?;
                writeln!(self.out)?;
            }
            // An empty CSV still has its header.
            Format::Csv if !self.header => writeln!(self.out, "{CSV_HEADER}")?,
            _ => {}
        }
        self.out.flush()
    }
}

/// The list of dig sites, with their groups (lot 7.2).
pub fn sites<W: Write>(format: Format, sites: &[SiteRecord], groups: &[SiteGroup], mut out: W) -> std::io::Result<()> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SiteOut<'a> {
        id: &'a str,
        name: &'a str,
        folders: &'a [String],
        groups: Vec<&'a str>,
        documents: u64,
        size: u64,
        last_indexed: Option<String>,
    }
    let list: Vec<SiteOut> = sites
        .iter()
        .map(|s| SiteOut {
            id: &s.id,
            name: &s.name,
            folders: &s.roots,
            groups: groups.iter().filter(|g| g.site_ids.contains(&s.id)).map(|g| g.name.as_str()).collect(),
            documents: s.doc_count,
            size: s.size_bytes,
            last_indexed: s.last_indexed.map(iso_date),
        })
        .collect();
    match format {
        Format::Text => {
            for s in &list {
                let when = s.last_indexed.as_deref().map_or_else(|| "never indexed".to_owned(), |d| format!("indexed {d}"));
                let groups = if s.groups.is_empty() { String::new() } else { format!(" [{}]", s.groups.join(", ")) };
                writeln!(out, "{}{groups}  ({} documents, {when})", s.name, s.documents)?;
                for folder in s.folders {
                    writeln!(out, "    {folder}")?;
                }
            }
        }
        Format::Json => {
            serde_json::to_writer_pretty(&mut out, &list)?;
            writeln!(out)?;
        }
        Format::Jsonl => {
            for s in &list {
                serde_json::to_writer(&mut out, s)?;
                writeln!(out)?;
            }
        }
        Format::Csv => {
            writeln!(out, "id,name,folders,groups,documents,size,lastIndexed")?;
            for s in &list {
                let fields = [
                    s.id.to_owned(),
                    s.name.to_owned(),
                    s.folders.join(";"),
                    s.groups.join(";"),
                    s.documents.to_string(),
                    s.size.to_string(),
                    s.last_indexed.clone().unwrap_or_default(),
                ];
                writeln!(out, "{}", fields.iter().map(|f| csv_field(f)).collect::<Vec<_>>().join(","))?;
            }
        }
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_both_ways() {
        assert_eq!(iso_date(1_710_498_620), "2024-03-15T10:30:20Z");
        assert_eq!(iso_date(0), "1970-01-01T00:00:00Z");
        assert_eq!(parse_date("2024-03-15"), Some(1_710_460_800));
        assert_eq!(parse_date("2024-02-29").map(iso_date).as_deref(), Some("2024-02-29T00:00:00Z"));
        assert_eq!(parse_date("2024-13-01"), None);
        assert_eq!(parse_date("hier"), None);
    }

    #[test]
    fn sizes() {
        assert_eq!(parse_size("500"), Ok(500));
        assert_eq!(parse_size("10KB"), Ok(10_240));
        assert_eq!(parse_size("2.5 mb"), Ok(2_621_440));
        assert!(parse_size("3 parsecs").is_err());
    }

    #[test]
    fn csv_quotes_when_needed() {
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("dit \"oui\""), "\"dit \"\"oui\"\"\"");
        assert_eq!(csv_field("simple"), "simple");
        assert_eq!(marks("le ⟦contrat⟧ et ⟪contrst⟫", "[", "]"), "le [contrat] et [contrst]");
    }
}
