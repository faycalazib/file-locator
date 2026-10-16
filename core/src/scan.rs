//! Live scan (goal.md §3 "non-indexed search"): walk the folders, read each
//! file and test it against the query on the fly. Slower than the index, but
//! needs no indexing and always sees the current content of the files.
//! Hits are streamed to `on_hit` as soon as they are found.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::sync_channel;
use std::time::{Instant, UNIX_EPOCH};

use serde::Serialize;

use crate::crawler::{crawl_each, crawl_folders, Exclusions, FileEntry};
use crate::disk::{Facts, HIDDEN};
use crate::error::Result;
use crate::extract::{is_container, INNER_SEP};
use crate::pipeline::{process_stream, SLOW_FILE};
use crate::highlight::snippets;
use crate::lang::{detect, DocLang};
use crate::names::NamePattern;
use crate::scope::{inside, restrict_targets};
use crate::search::{Hit, QueryLogic, SearchRequest};

/// Files found by the walk and not read yet (bounded: memory stays small).
const WALK_AHEAD: usize = 4096;

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

/// Scans the targets. `on_progress(read, found)` is throttled: files read,
/// and files found so far by the walk (which goes on while reading).
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
    // Files found by the walk (and kept by the filters): what is left to read.
    let walked = AtomicUsize::new(0);
    let found = AtomicUsize::new(0);
    let limit = req.limit.unwrap_or(1000);

    for target in targets {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        // Files are read as the walk finds them (BUG-073): on a whole disk,
        // the first results come in seconds, not after the full listing.
        let keep = |f: &FileEntry| {
            // Dates and attributes come with the walk: an archive's entries share them.
            disk.as_ref().is_none_or(|d| d.accepts_facts(&f.facts))
                // Kind filter before reading anything (archives may hold any kind).
                && (req.kinds.is_empty() || f.kind.as_str() == "archive" || req.kinds.iter().any(|k| k == f.kind.as_str()))
                && names.as_ref().is_none_or(|n| (is_container(&f.path, f.kind) && !whole_files) || n.matches(&file_name(&f.path)))
                && (req.in_files.is_empty() || req.in_files.iter().any(|p| Path::new(p) == f.path))
                && within_files.as_ref().is_none_or(|keep| keep.contains(f.path.to_string_lossy().as_ref()))
        };
        // Name only (and no language filter, which needs the text): nothing
        // to read in plain files.
        let name_only = parsed.is_empty() && req.langs.is_empty();
        let (tx, rx) = sync_channel::<FileEntry>(WALK_AHEAD);
        std::thread::scope(|scope| {
            let (walked, keep, exclusions, scanned) = (&walked, &keep, &exclusions, &scanned);
            scope.spawn(move || {
                crawl_each(&target.roots, exclusions, hidden, cancel, &|file| {
                    if !keep(&file) {
                        return;
                    }
                    let n = walked.fetch_add(1, Ordering::Relaxed) + 1;
                    if n.is_multiple_of(256) {
                        on_progress(scanned.load(Ordering::Relaxed), n);
                    }
                    let _ = tx.send(file);
                });
            });
            let to_read = rx.into_iter().filter_map(|file| {
                if !name_only || (!whole_files && is_container(&file.path, file.kind)) {
                    return Some(file);
                }
                if found.load(Ordering::Relaxed) >= limit {
                    cancel_if_full(cancel);
                    return None;
                }
                let path = file.path.to_string_lossy().into_owned();
                if passes_filters(req, file.kind.as_str(), file.size, file.modified, file.facts.created)
                    && within.as_ref().is_none_or(|w| w.contains(path.as_str()))
                    && digest.as_ref().is_none_or(|d| d.matches(&file.path))
                {
                    found.fetch_add(1, Ordering::Relaxed);
                    on_hit(name_hit(&target.site_id, path, file.kind.as_str(), file.size, file.modified, file.facts.created));
                }
                count(scanned, walked, on_progress);
                None
            });
            process_stream(to_read, cancel, SLOW_FILE, |_| {}, |file, extracted| {
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
                        // Its own name counts, as in the index, where every word
                        // or phrase is looked for in `body` and `name` (BUG-077);
                        // not with LINES, which counts the lines of the text.
                        let own_name = doc.inner.as_deref().map_or(container.as_str(), inner_name);
                        let accepted = if parsed.lines.is_some() {
                            logic.accepts(&doc.text, lang)
                        } else {
                            logic.accepts(&format!("{own_name}\n{}", doc.text), lang)
                        };
                        if !accepted {
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
                            sensitive: highlighter.sensitive(&doc.text),
                            match_count: matches.len(),
                            exact_count,
                            // Live results arrive in disk order: more matches first when sorting.
                            score: exact_count as f32 + 0.1 * matches.len() as f32,
                            snippets: snippets(&doc.text, &matches, 2),
                            inner_kind: None,
                            meaning: None,
                        }
                        .with_inner_kind());
                    }
                }
                count(scanned, walked, on_progress);
            });
        });
    }
    on_progress(scanned.load(Ordering::Relaxed), walked.load(Ordering::Relaxed));

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

/// Folders whose name matches, taken from the paths of a site's indexed files
/// (BUG-078): the disk is not walked, only the folders found are read on it
/// (date, attributes). A folder holding no indexed file is not seen: the live
/// scan ([`find_folders`]) walks the disk for those.
pub fn folders_from_files<'a>(
    site_id: &str,
    roots: &[String],
    files: impl Iterator<Item = &'a str>,
    excluded: &[String],
    req: &SearchRequest,
) -> Result<Vec<Hit>> {
    let names = match req.names()? {
        Some(names) => Some(names),
        None => NamePattern::parse(&req.query)?,
    };
    let Some(names) = names else {
        return Ok(Vec::new());
    };
    // Every folder between a root (excluded) and its files, once.
    let mut seen = HashSet::new();
    let mut matching = BTreeSet::new();
    for file in files {
        let Some(root) = roots.iter().find(|root| inside(root, file).is_some_and(|tail| !tail.is_empty())) else {
            continue;
        };
        for folder in Path::new(file).ancestors().skip(1) {
            let path = folder.to_string_lossy();
            if inside(root, &path).is_none_or(str::is_empty) || !seen.insert(path.to_string()) {
                // The root, or a folder already seen with all its parents.
                break;
            }
            if folder.file_name().is_some_and(|name| names.matches(&name.to_string_lossy())) {
                matching.insert(path.into_owned());
            }
        }
    }
    let exclusions = Exclusions::new(excluded);
    let within = req.within();
    let disk = req.disk_check()?;
    let limit = req.limit.unwrap_or(1000);
    let mut hits = Vec::new();
    for path in matching {
        if hits.len() >= limit {
            break;
        }
        let in_folder = req.folder().is_none_or(|f| inside(f, &path).is_some_and(|tail| !tail.is_empty()));
        if !in_folder || exclusions.is_excluded(Path::new(&path)) || within.as_ref().is_some_and(|w| !w.contains(path.as_str())) {
            continue;
        }
        // Gone since the last update: not listed.
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        let modified = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
        let facts = Facts::from_meta(Path::new(&path), &meta);
        if !req.date_ok(modified, facts.created) || disk.as_ref().is_some_and(|d| !d.accepts_facts(&facts)) {
            continue;
        }
        hits.push(name_hit(site_id, path, "folder", 0, modified, facts.created));
    }
    Ok(hits)
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
        sensitive: Vec::new(),
        match_count: 0,
        exact_count: 0,
        score: 1.0,
        snippets: Vec::new(),
        inner_kind: None,
        meaning: None,
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

/// One more file read; `on_progress(read, found by the walk)` every 50.
fn count(scanned: &AtomicUsize, walked: &AtomicUsize, on_progress: &(dyn Fn(usize, usize) + Sync)) {
    let n = scanned.fetch_add(1, Ordering::Relaxed) + 1;
    if n.is_multiple_of(50) {
        on_progress(n, walked.load(Ordering::Relaxed));
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

    /// BUG-078: a folder at the first level of the site (`copy-app`) and a
    /// deeper one are found from the files' paths; the root is not a result.
    #[test]
    fn folders_from_the_paths_of_indexed_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("GIT Repositories");
        for sub in ["copy-app/src", "other/copy-app-old", "other/docs"] {
            std::fs::create_dir_all(root.join(sub)).unwrap();
        }
        let file = |rel: &str| root.join(rel).to_string_lossy().into_owned();
        let files = [file("copy-app/src/main.ts"), file("copy-app/README.md"), file("other/copy-app-old/a.txt"), file("other/docs/b.txt")];
        let roots = [root.to_string_lossy().into_owned()];
        let found = |req: &SearchRequest, excluded: &[String]| -> Vec<String> {
            folders_from_files("s", &roots, files.iter().map(String::as_str), excluded, req)
                .unwrap()
                .into_iter()
                .map(|h| h.path.strip_prefix(roots[0].as_str()).unwrap().replace('\\', "/"))
                .collect()
        };
        let named = SearchRequest { name_pattern: "copy-app".into(), folders: true, ..Default::default() };
        assert_eq!(found(&named, &[]), ["/copy-app", "/other/copy-app-old"]);
        let all = SearchRequest { name_pattern: "*".into(), folders: true, ..Default::default() };
        assert_eq!(found(&all, &[]), ["/copy-app", "/copy-app/src", "/other", "/other/copy-app-old", "/other/docs"], "root not listed");
        // Excluded since the last update, or limited to a folder.
        assert_eq!(found(&named, &[file("other")]), ["/copy-app"]);
        let in_other = SearchRequest { in_folder: Some(file("other")), ..all.clone() };
        assert_eq!(found(&in_other, &[]), ["/other/copy-app-old", "/other/docs"]);
        // Removed from the disk since: not listed.
        std::fs::remove_dir_all(root.join("other/docs")).unwrap();
        assert_eq!(found(&all, &[]), ["/copy-app", "/copy-app/src", "/other", "/other/copy-app-old"]);
    }

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
