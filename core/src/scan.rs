//! Live scan (goal.md §3 "non-indexed search"): walk the folders, read each
//! file and test it against the query on the fly. Slower than the index, but
//! needs no indexing and always sees the current content of the files.
//! Hits are streamed to `on_hit` as soon as they are found.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use serde::Serialize;

use crate::crawler::{crawl_folders, crawl_with, Exclusions, FileEntry};
use crate::disk::HIDDEN;
use crate::error::Result;
use crate::extract::{is_container, INNER_SEP};
use crate::pipeline::process_files;
use crate::highlight::snippets;
use crate::lang::{detect, DocLang};
use crate::names::NamePattern;
use crate::scope::restrict_targets;
use crate::search::{Hit, QueryLogic, SearchRequest};

/// Where to scan: a dig site's roots, labelled with its id.
pub struct ScanTarget {
    pub site_id: String,
    pub roots: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub files_scanned: usize,
    pub hits: usize,
    pub took_ms: u64,
    pub cancelled: bool,
}

/// Scans the targets. `on_progress(scanned, total)` is throttled.
///
/// The file-name criterion is applied first, so the other files are never
/// read. Without text to look for, a plain file is answered from its name
/// alone; archives and mailboxes are still opened to check their entries.
pub fn live_scan(
    targets: &[ScanTarget],
    excluded: &[String],
    req: &SearchRequest,
    cancel: &AtomicBool,
    on_hit: &(dyn Fn(Hit) + Sync),
    on_progress: &(dyn Fn(usize, usize) + Sync),
) -> Result<ScanSummary> {
    let started = Instant::now();
    // Limited to a folder (Explorer right-click): that folder only, scope.rs.
    let restricted;
    let targets = match req.folder() {
        Some(folder) => {
            restricted = restrict_targets(targets, folder);
            &restricted[..]
        }
        None => targets,
    };
    if req.folders {
        return find_folders(targets, excluded, req, cancel, on_hit);
    }
    let parsed = req.parsed();
    let names = req.names()?;
    let highlighter = req.matcher(&parsed)?;
    let logic = QueryLogic::new(&parsed, req)?;
    let exclusions = Exclusions::new(excluded);
    // Last access, attributes, digest (lot 5.8). Asked for hidden files, the
    // walk goes through them too.
    let disk = req.disk_check()?;
    let hidden = disk.as_ref().is_some_and(|d| d.attributes & HIDDEN != 0);
    let digest = disk.as_ref().and_then(|d| d.digest.clone());
    // A digest and no text: every file is answered as a whole (an archive
    // included), nothing is extracted.
    let whole_files = digest.is_some() && parsed.is_empty() && req.langs.is_empty();
    // Search within results: only the files holding those documents are read.
    let within = req.within();
    let within_files: Option<std::collections::HashSet<&str>> =
        within.as_ref().map(|w| w.iter().map(|p| p.split(INNER_SEP).next().unwrap_or(p)).collect());
    let scanned = AtomicUsize::new(0);
    let found = AtomicUsize::new(0);
    let limit = req.limit.unwrap_or(1000);

    for target in targets {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let mut files = crawl_with(&target.roots, &exclusions, hidden, cancel, &|_| {}).files;
        // Dates and attributes come with the walk: an archive's entries share them.
        if let Some(disk) = &disk {
            files.retain(|f| disk.accepts_facts(&f.facts));
        }
        // Kind filter before reading anything (archives may hold any kind).
        if !req.kinds.is_empty() {
            files.retain(|f| f.kind.as_str() == "archive" || req.kinds.iter().any(|k| k == f.kind.as_str()));
        }
        if let Some(names) = &names {
            files.retain(|f| (is_container(&f.path, f.kind) && !whole_files) || names.matches(&file_name(&f.path)));
        }
        if !req.in_files.is_empty() {
            files.retain(|f| req.in_files.iter().any(|p| Path::new(p) == f.path));
        }
        if let Some(keep) = &within_files {
            files.retain(|f| keep.contains(f.path.to_string_lossy().as_ref()));
        }
        let total = files.len();
        // Name only (and no language filter, which needs the text): nothing
        // to read in plain files.
        if parsed.is_empty() && req.langs.is_empty() {
            let (plain, containers): (Vec<FileEntry>, Vec<FileEntry>) =
                files.into_iter().partition(|f| whole_files || !is_container(&f.path, f.kind));
            for file in plain {
                if found.load(Ordering::Relaxed) >= limit {
                    cancel_if_full(cancel);
                    break;
                }
                let path = file.path.to_string_lossy().into_owned();
                if passes_filters(req, file.kind.as_str(), file.size, file.modified, file.facts.created)
                    && within.as_ref().is_none_or(|w| w.contains(path.as_str()))
                    && digest.as_ref().is_none_or(|d| d.matches(&file.path))
                {
                    found.fetch_add(1, Ordering::Relaxed);
                    on_hit(name_hit(&target.site_id, path, file.kind.as_str(), file.size, file.modified, file.facts.created));
                }
                count(&scanned, total, on_progress);
            }
            files = containers;
        }
        process_files(&mut files, cancel, |file, extracted| {
            if found.load(Ordering::Relaxed) >= limit {
                cancel_if_full(cancel);
                return;
            }
            if let Ok(docs) = extracted {
                let file_path = file.path.to_string_lossy();
                let container = file_name(&file.path);
                for doc in docs {
                    // An entry of an archive or a mailbox answers to its own
                    // name or to its container's (same rule as the index).
                    // A message (.msg, .eml) is also a document of its own,
                    // without inner path: it answers to its file name.
                    if let Some(names) = &names {
                        let accepted = match &doc.inner {
                            Some(inner) => names.matches_any(&[inner_name(inner), &container]),
                            None => names.matches(&container),
                        };
                        if !accepted {
                            continue;
                        }
                    }
                    if let Some(within) = &within {
                        let full = match &doc.inner {
                            Some(inner) => format!("{file_path}{INNER_SEP}{inner}"),
                            None => file_path.clone().into_owned(),
                        };
                        if !within.contains(full.as_str()) {
                            continue;
                        }
                    }
                    let size = doc.size.unwrap_or(file.size);
                    let modified = doc.modified.unwrap_or(file.modified);
                    if !passes_filters(req, doc.kind.as_str(), size, modified, file.facts.created) {
                        continue;
                    }
                    let lang = detect(&doc.text);
                    if !req.langs.is_empty() && !req.langs.iter().any(|l| l == lang.code()) {
                        continue;
                    }
                    if !logic.accepts(&doc.text, lang) {
                        continue;
                    }
                    // Last: the digest of the file (an inner document has none).
                    if digest.as_ref().is_some_and(|d| doc.inner.is_some() || !d.matches(&file.path)) {
                        continue;
                    }
                    let matches = highlighter.find(&doc.text, lang);
                    let path = match &doc.inner {
                        Some(inner) => format!("{file_path}{INNER_SEP}{inner}"),
                        None => file_path.clone().into_owned(),
                    };
                    let exact_count = matches.iter().filter(|m| !m.fuzzy).count();
                    found.fetch_add(1, Ordering::Relaxed);
                    on_hit(Hit {
                        site_id: target.site_id.clone(),
                        path,
                        kind: doc.kind.as_str().to_owned(),
                        lang: lang.code().to_owned(),
                        size_bytes: size,
                        modified,
                        created: file.facts.created,
                        detections: highlighter.detections(&doc.text),
                        match_count: matches.len(),
                        exact_count,
                        // Live results arrive in disk order: more matches first when sorting.
                        score: exact_count as f32 + 0.1 * matches.len() as f32,
                        snippets: snippets(&doc.text, &matches, 2),
                        inner_kind: None,
                    }
                    .with_inner_kind());
                }
            }
            count(&scanned, total, on_progress);
        });
    }

    Ok(ScanSummary {
        files_scanned: scanned.load(Ordering::Relaxed),
        hits: found.load(Ordering::Relaxed),
        took_ms: started.elapsed().as_millis() as u64,
        cancelled: cancel.load(Ordering::Relaxed),
    })
}

/// Folders whose name matches the file-name criterion (or, without one, the
/// text typed in the search box). Folders are not in the indexes: the disk is
/// walked, which only reads folder entries, never files.
pub fn find_folders(
    targets: &[ScanTarget],
    excluded: &[String],
    req: &SearchRequest,
    cancel: &AtomicBool,
    on_hit: &(dyn Fn(Hit) + Sync),
) -> Result<ScanSummary> {
    let started = Instant::now();
    let names = match req.names()? {
        Some(names) => Some(names),
        None => NamePattern::parse(&req.query)?,
    };
    let Some(names) = names else {
        return Ok(ScanSummary { files_scanned: 0, hits: 0, took_ms: 0, cancelled: false });
    };
    let exclusions = Exclusions::new(excluded);
    let within = req.within();
    let disk = req.disk_check()?;
    let limit = req.limit.unwrap_or(1000);
    let mut found = 0;
    for target in targets {
        let folders = crawl_folders(&target.roots, &exclusions, cancel, &|name| names.matches(name));
        for folder in folders {
            if found >= limit {
                break;
            }
            let path = folder.path.to_string_lossy().into_owned();
            if !req.date_ok(folder.modified, folder.facts.created)
                || disk.as_ref().is_some_and(|d| !d.accepts_facts(&folder.facts))
                || within.as_ref().is_some_and(|w| !w.contains(path.as_str()))
            {
                continue;
            }
            found += 1;
            on_hit(name_hit(&target.site_id, path, "folder", 0, folder.modified, folder.facts.created));
        }
    }
    Ok(ScanSummary {
        files_scanned: found,
        hits: found,
        took_ms: started.elapsed().as_millis() as u64,
        cancelled: cancel.load(Ordering::Relaxed),
    })
}

/// A result found by its name only: no text, no snippet.
fn name_hit(site_id: &str, path: String, kind: &str, size: u64, modified: u64, created: u64) -> Hit {
    Hit {
        site_id: site_id.to_owned(),
        path,
        kind: kind.to_owned(),
        lang: DocLang::Und.code().to_owned(),
        size_bytes: size,
        modified,
        created,
        detections: Default::default(),
        match_count: 0,
        exact_count: 0,
        score: 1.0,
        snippets: Vec::new(),
        inner_kind: None,
    }
    .with_inner_kind()
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Last part of an inner path (`dossier/contrat.pdf` → `contrat.pdf`).
fn inner_name(inner: &str) -> &str {
    inner.rsplit(['/', '\\', '›']).next().unwrap_or(inner).trim()
}

/// The hit limit is reached: stop reading the remaining files.
fn cancel_if_full(cancel: &AtomicBool) {
    cancel.store(true, Ordering::Relaxed);
}

fn count(scanned: &AtomicUsize, total: usize, on_progress: &(dyn Fn(usize, usize) + Sync)) {
    let n = scanned.fetch_add(1, Ordering::Relaxed) + 1;
    if n.is_multiple_of(50) || n == total {
        on_progress(n, total);
    }
}

fn passes_filters(req: &SearchRequest, kind: &str, size: u64, modified: u64, created: u64) -> bool {
    (req.kinds.is_empty() || req.kinds.iter().any(|k| k == kind))
        && req.min_size.is_none_or(|min| size >= min)
        && req.max_size.is_none_or(|max| size < max)
        && req.date_ok(modified, created)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::parse;

    #[test]
    fn boolean_logic_on_one_text() {
        let req = SearchRequest::default();
        let logic = QueryLogic::new(&parse("contrat OR contrato NOT brouillon"), &req).unwrap();
        assert!(logic.accepts("Le contrat signé.", DocLang::Fr));
        assert!(logic.accepts("El contrato firmado.", DocLang::Es));
        assert!(!logic.accepts("Brouillon du contrat.", DocLang::Fr));
        assert!(!logic.accepts("Une facture.", DocLang::Fr));
    }
}
