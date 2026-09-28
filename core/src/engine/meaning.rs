//! The meaning index of the sites (lot 8.2), computed from the text already
//! in each site's index (nothing read on the disk again). What is done is
//! kept in `indexes/<id>.sense.json` (the file's stamp in the main manifest,
//! and its passages): only new or changed files are computed, and an
//! interrupted computation starts again where it stopped.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tantivy::collector::DocSetCollector;
use tantivy::query::TermQuery;
use tantivy::schema::{IndexRecordOption, Value};
use tantivy::{TantivyDocument, Term};

use super::{lock, Engine};
use crate::error::Result;
use crate::extract::{inner_name, split_inner};
use crate::fsutil::write_atomic;
use crate::kind::FileKind;
use crate::manifest::Manifest;
use crate::sense::chunk::{count, passages};
use crate::highlight::Snippet;
use crate::search::{stored_body, Hit, MeaningMatch, SearchRequest};
use crate::sense::store::{PassageHit, SenseIndex};
use crate::sense::DIM;
use crate::SearchResponse;

/// Passages looked at for a search by meaning.
const MEANING_PASSAGES: usize = 400;
/// Documents found by meaning, at most.
const MEANING_DOCUMENTS: usize = 60;
/// Scores are close together (an unrelated text is at about 0.77 when a good
/// one is at 0.80): kept, the documents within this of the best one.
const MEANING_MARGIN: f32 = 0.05;
/// Reciprocal rank fusion (words + meaning): the usual constant.
const RRF_K: f32 = 60.0;

/// A passage's range in a text, on character boundaries.
pub(super) fn passage_range(body: &str, start: usize, end: usize) -> Option<std::ops::Range<usize>> {
    let fix = |mut i: usize| {
        i = i.min(body.len());
        while !body.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    let (a, b) = (fix(start), fix(end));
    (a < b).then_some(a..b)
}

/// Format of the meaning manifest (a new one computes everything again).
const VERSION: u32 = 1;
/// Files read from the main index at a time; committed together.
const FILES_PER_BATCH: usize = 32;
/// Passages given to the models at a time.
const PASSAGES_PER_CALL: usize = 32;
/// Tries of a batch's commit while Windows answers "access denied" (BUG-036).
const COMMIT_ATTEMPTS: u32 = 5;

/// A new index file briefly locked by the antivirus (BUG-022, BUG-036).
fn is_access_denied(e: &crate::error::CoreError) -> bool {
    let text = e.to_string();
    text.contains("PermissionDenied") || text.contains("Access is denied")
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct SenseStamp {
    size: u64,
    modified: u64,
    passages: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct SenseManifest {
    version: u32,
    files: BTreeMap<String, SenseStamp>,
}

impl Default for SenseManifest {
    fn default() -> Self {
        Self { version: VERSION, files: BTreeMap::new() }
    }
}

/// What remains to compute for a site.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SensePlan {
    /// Files to compute (new or changed).
    pub files: usize,
    /// Files whose passages go (removed, or now without text).
    pub removed: usize,
    /// Passages to compute (estimated from the stored text), when asked.
    pub passages: usize,
    /// Passages already in the meaning index.
    pub done: u64,
}

/// A passage found by its meaning, and its site.
#[derive(Clone, Debug, PartialEq)]
pub struct SenseHit {
    pub site_id: String,
    pub hit: PassageHit,
}

/// Progress of a computation: passages done / to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SenseProgress {
    pub done: usize,
    pub total: usize,
}

/// Turns passages into vectors (the models, in the background).
pub type Embed<'a> = dyn Fn(&[String]) -> Result<Vec<[f32; DIM]>> + 'a;

/// A document of the main index to cut into passages.
struct SourceDoc {
    path: String,
    file: String,
    name: String,
    body: String,
}

/// The documents worth understanding: everything but source code.
fn eligible_file(path: &str) -> bool {
    FileKind::from_path(std::path::Path::new(path)) != Some(FileKind::Code)
}

impl Engine {
    fn sense_dir(&self, id: &str) -> PathBuf {
        self.data_dir.join("indexes").join(format!("{id}.sense"))
    }

    fn sense_manifest_path(&self, id: &str) -> PathBuf {
        self.data_dir.join("indexes").join(format!("{id}.sense.json"))
    }

    fn sense_manifest(&self, id: &str) -> SenseManifest {
        std::fs::read(self.sense_manifest_path(id))
            .ok()
            .and_then(|b| serde_json::from_slice::<SenseManifest>(&b).ok())
            .filter(|m| m.version == VERSION)
            .unwrap_or_default()
    }

    fn sense_index(&self, id: &str) -> Result<Arc<SenseIndex>> {
        let mut open = lock(&self.sense_open);
        if let Some(index) = open.get(id) {
            return Ok(index.clone());
        }
        let index = Arc::new(SenseIndex::open(&self.sense_dir(id), self.reader_of().is_some())?);
        open.insert(id.to_owned(), index.clone());
        Ok(index)
    }

    /// Turns the meaning index of a site on or off (off: it is deleted).
    pub fn set_site_sense(&self, id: &str, on: bool) -> Result<super::SiteRecord> {
        self.writable()?;
        let site = self.update_site(id, |s| s.sense = on)?;
        if !on {
            self.forget_sense(id)?;
        }
        Ok(site)
    }

    /// Deletes the meaning index of a site (turned off, or site removed).
    pub(super) fn forget_sense(&self, id: &str) -> Result<()> {
        lock(&self.sense_open).remove(id);
        let dir = self.sense_dir(id);
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
        let manifest = self.sense_manifest_path(id);
        if manifest.exists() {
            std::fs::remove_file(manifest)?;
        }
        Ok(())
    }

    /// Files to compute and to remove: the main manifest against the
    /// meaning manifest (nothing is read but these two files).
    fn sense_todo(&self, id: &str) -> (Vec<(String, u64, u64)>, Vec<String>) {
        let Some(main) = Manifest::load(&self.manifest_path(id)) else { return (Vec::new(), Vec::new()) };
        let done = self.sense_manifest(id);
        let mut todo = Vec::new();
        let mut keep = HashSet::new();
        for (path, stamp) in &main.files {
            if stamp.skipped.is_some() || stamp.docs == 0 || !eligible_file(path) {
                continue;
            }
            keep.insert(path.as_str());
            let same = done.files.get(path).is_some_and(|d| d.size == stamp.size && d.modified == stamp.modified);
            if !same {
                todo.push((path.clone(), stamp.size, stamp.modified));
            }
        }
        let removed = done.files.keys().filter(|p| !keep.contains(String::as_str(p))).cloned().collect();
        (todo, removed)
    }

    /// What remains to compute. `estimate`: also count the passages (reads
    /// the stored text of the files to compute).
    pub fn sense_plan(&self, id: &str, estimate: bool) -> Result<SensePlan> {
        let (todo, removed) = self.sense_todo(id);
        let done = if self.sense_dir(id).exists() { self.sense_index(id)?.len() } else { 0 };
        let mut plan = SensePlan { files: todo.len(), removed: removed.len(), passages: 0, done };
        if estimate && !todo.is_empty() {
            let wanted: HashSet<&str> = todo.iter().map(|(p, _, _)| p.as_str()).collect();
            let index = self.site_index(id)?;
            let searcher = index.reader.searcher();
            let f = index.fields;
            for segment in searcher.segment_readers() {
                let store = segment.get_store_reader(64)?;
                for doc_id in segment.doc_ids_alive() {
                    let doc: TantivyDocument = store.get(doc_id)?;
                    let path = doc.get_first(f.path).and_then(|v| v.as_str()).unwrap_or_default();
                    if !wanted.contains(split_inner(path).0) || !eligible_doc(path) {
                        continue;
                    }
                    plan.passages += count(doc.get_first(f.body).and_then(|v| v.as_str()).unwrap_or_default());
                }
            }
        }
        Ok(plan)
    }

    /// The documents of these files, read from the main index.
    fn source_docs(&self, id: &str, files: &[String]) -> Result<Vec<SourceDoc>> {
        let index = self.site_index(id)?;
        let searcher = index.reader.searcher();
        let f = index.fields;
        let mut out = Vec::new();
        for file in files {
            let query = TermQuery::new(Term::from_field_text(f.file, file), IndexRecordOption::Basic);
            let mut addresses: Vec<_> = searcher.search(&query, &DocSetCollector)?.into_iter().collect();
            addresses.sort();
            for address in addresses {
                let doc: TantivyDocument = searcher.doc(address)?;
                let text = |field| doc.get_first(field).and_then(|v| v.as_str()).unwrap_or_default().to_owned();
                let path = text(f.path);
                if !eligible_doc(&path) {
                    continue;
                }
                let (disk, inner) = split_inner(&path);
                let name = match inner {
                    Some(inner) => inner_name(inner).to_owned(),
                    None => std::path::Path::new(disk).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                };
                out.push(SourceDoc { path: path.clone(), file: file.clone(), name, body: text(f.body) });
            }
        }
        Ok(out)
    }

    /// Computes what remains of a site's meaning index, by batches committed
    /// as they go: stopped (`cancel`), it starts again from there. `embed`
    /// turns passages into vectors (the models, in the background).
    /// Returns true when everything is done.
    pub fn sense_update(
        &self,
        id: &str,
        embed: &Embed<'_>,
        cancel: &AtomicBool,
        progress: &dyn Fn(SenseProgress),
    ) -> Result<bool> {
        self.writable()?;
        let (todo, removed) = self.sense_todo(id);
        if todo.is_empty() && removed.is_empty() {
            return Ok(true);
        }
        let plan = self.sense_plan(id, true)?;
        let index = self.sense_index(id)?;
        let mut manifest = self.sense_manifest(id);
        let mut writer = index.writer()?;
        for file in &removed {
            index.delete_file(&writer, file);
            manifest.files.remove(file);
        }
        writer.commit()?;
        write_atomic(&self.sense_manifest_path(id), &serde_json::to_vec(&manifest)?)?;

        let mut done = 0usize;
        progress(SenseProgress { done, total: plan.passages });
        for batch in todo.chunks(FILES_PER_BATCH) {
            if cancel.load(Ordering::Relaxed) {
                index.reader.reload()?;
                return Ok(false);
            }
            let files: Vec<String> = batch.iter().map(|(p, _, _)| p.clone()).collect();
            let docs = self.source_docs(id, &files)?;
            // (document, passage number, place, text) of the whole batch.
            let mut pending: Vec<(usize, usize, (usize, usize), String)> = Vec::new();
            let mut per_file: BTreeMap<&str, u32> = BTreeMap::new();
            for (d, doc) in docs.iter().enumerate() {
                for (n, p) in passages(&doc.name, &doc.body).into_iter().enumerate() {
                    pending.push((d, n, (p.start, p.end), p.text));
                    *per_file.entry(doc.file.as_str()).or_default() += 1;
                }
            }
            // The vectors first (the long part), nothing written yet: a stop
            // drops the batch, done again next time.
            let mut vectors: Vec<[f32; DIM]> = Vec::with_capacity(pending.len());
            for chunk in pending.chunks(PASSAGES_PER_CALL) {
                if cancel.load(Ordering::Relaxed) {
                    index.reader.reload()?;
                    return Ok(false);
                }
                let texts: Vec<String> = chunk.iter().map(|(_, _, _, t)| t.clone()).collect();
                vectors.extend(embed(&texts)?);
                done += chunk.len();
                progress(SenseProgress { done, total: plan.passages.max(done) });
            }
            // Then written and committed; an antivirus briefly locking a new
            // index file (BUG-036) is waited for, the vectors are kept.
            let mut attempt = 1;
            loop {
                let written = (|| -> Result<()> {
                    for file in &files {
                        index.delete_file(&writer, file);
                    }
                    for ((d, n, place, _), v) in pending.iter().zip(&vectors) {
                        let doc = &docs[*d];
                        index.add(&writer, &doc.path, &doc.file, *n, *place, v)?;
                    }
                    writer.commit()?;
                    Ok(())
                })();
                match written {
                    Ok(()) => break,
                    Err(e) if attempt < COMMIT_ATTEMPTS && is_access_denied(&e) => {
                        attempt += 1;
                        let _ = writer.rollback();
                        drop(writer);
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        writer = index.writer()?;
                    }
                    Err(e) => return Err(e),
                }
            }
            for (file, size, modified) in batch {
                let passages = per_file.get(file.as_str()).copied().unwrap_or(0);
                manifest.files.insert(file.clone(), SenseStamp { size: *size, modified: *modified, passages });
            }
            write_atomic(&self.sense_manifest_path(id), &serde_json::to_vec(&manifest)?)?;
        }
        writer.wait_merging_threads()?;
        index.reader.reload()?;
        progress(SenseProgress { done, total: done });
        Ok(true)
    }

    /// Passages already in a site's meaning index (the rail).
    pub fn sense_passages(&self, id: &str) -> u64 {
        if !self.sense_dir(id).exists() {
            return 0;
        }
        self.sense_index(id).map(|index| index.len()).unwrap_or(0)
    }

    /// Whether a site has something left to compute (cheap: manifests only).
    pub fn sense_pending(&self, id: &str) -> bool {
        let (todo, removed) = self.sense_todo(id);
        !todo.is_empty() || !removed.is_empty()
    }

    /// The passages closest to `query` in these sites, best first.
    pub fn sense_search(&self, site_ids: &[String], query: &[f32; DIM], limit: usize) -> Result<Vec<SenseHit>> {
        let mut hits = Vec::new();
        for id in site_ids {
            if !self.sense_dir(id).exists() {
                continue;
            }
            let index = self.sense_index(id)?;
            if self.reader_of().is_some() {
                let _ = index.reader.reload();
            }
            hits.extend(index.search(query, limit)?.into_iter().map(|hit| SenseHit { site_id: id.clone(), hit }));
        }
        hits.sort_by(|a, b| b.hit.score.total_cmp(&a.hit.score));
        hits.truncate(limit);
        Ok(hits)
    }

    /// Search by words and by meaning (lot 8.3). `question`: the vector of
    /// the question (computed by the app). The documents closest in meaning
    /// go through the same filters as a search by name, then both lists are
    /// fused (reciprocal ranks); a document found by meaning only shows its
    /// closest passage instead of words.
    pub fn search_meaning(&self, site_ids: &[String], req: &SearchRequest, question: &[f32; DIM]) -> Result<SearchResponse> {
        let started = std::time::Instant::now();
        let by_words = if req.is_empty() { None } else { Some(self.search(site_ids, req)?) };

        // Closest documents: the best passage of each, near the best score.
        let mut best: Vec<SenseHit> = Vec::new();
        for hit in self.sense_search(site_ids, question, MEANING_PASSAGES)? {
            if !best.iter().any(|b| b.site_id == hit.site_id && b.hit.path == hit.hit.path) {
                best.push(hit);
            }
        }
        let top = best.first().map_or(0.0, |b| b.hit.score);
        best.retain(|b| b.hit.score >= top - MEANING_MARGIN);
        best.truncate(MEANING_DOCUMENTS);

        // The filters of the search, applied to them (by name: every name).
        let candidates: Vec<String> = best.iter().map(|b| b.hit.path.clone()).collect();
        let within = if req.within_paths.is_empty() {
            candidates.clone()
        } else {
            candidates.iter().filter(|p| req.within_paths.contains(p)).cloned().collect()
        };
        let mut filtered: Vec<Hit> = Vec::new();
        if !within.is_empty() {
            let by_name = SearchRequest {
                query: String::new(),
                term_list: Vec::new(),
                name_pattern: if req.name_pattern.trim().is_empty() { "*".to_owned() } else { req.name_pattern.clone() },
                within_paths: within,
                facets: false,
                limit: Some(MEANING_DOCUMENTS),
                ..req.clone()
            };
            filtered = self.search(site_ids, &by_name)?.hits;
        }

        // Fusion: 1 / (k + rank) in each list.
        let mut fused: Vec<(f32, Hit)> = Vec::new();
        let words: Vec<Hit> = by_words.as_ref().map(|r| r.hits.clone()).unwrap_or_default();
        for (rank, hit) in words.into_iter().enumerate() {
            fused.push((1.0 / (RRF_K + rank as f32 + 1.0), hit));
        }
        let mut only = 0usize;
        let mut rank = 0usize;
        for sense in &best {
            let Some(found) = filtered.iter().find(|h| h.site_id == sense.site_id && h.path == sense.hit.path) else { continue };
            rank += 1;
            let bonus = 1.0 / (RRF_K + rank as f32);
            let meaning = MeaningMatch { score: sense.hit.score, start: sense.hit.start, end: sense.hit.end, only: false };
            if let Some((score, hit)) = fused.iter_mut().find(|(_, h)| h.site_id == found.site_id && h.path == found.path) {
                *score += bonus;
                hit.meaning = Some(meaning);
                continue;
            }
            let mut hit = found.clone();
            hit.meaning = Some(MeaningMatch { only: true, ..meaning });
            hit.match_count = 0;
            hit.exact_count = 0;
            hit.snippets = self.passage_snippet(&sense.site_id, &hit.path, sense.hit.start, sense.hit.end).into_iter().collect();
            fused.push((bonus, hit));
            only += 1;
        }
        fused.sort_by(|a, b| b.0.total_cmp(&a.0));
        fused.truncate(req.limit.unwrap_or(200).max(1));
        let mut response = by_words.unwrap_or(SearchResponse {
            hits: Vec::new(),
            total_files: 0,
            exact_files: 0,
            took_ms: 0,
            detections: Default::default(),
            facets: None,
        });
        response.total_files += only;
        response.hits = fused.into_iter().map(|(score, mut hit)| {
            hit.score = score;
            hit
        }).collect();
        response.took_ms = started.elapsed().as_millis() as u64;
        Ok(response)
    }

    /// The passage found by meaning, as the snippet of its result.
    fn passage_snippet(&self, site_id: &str, path: &str, start: usize, end: usize) -> Option<Snippet> {
        let index = self.site_index(site_id).ok()?;
        let (body, _, _) = stored_body(&index, path).ok()??;
        let range = passage_range(&body, start, end)?;
        let line = body[..range.start].matches('\n').count() + 1;
        let text: String = body[range].split_whitespace().collect::<Vec<_>>().join(" ");
        let text = if text.chars().count() > 320 { format!("{}…", text.chars().take(320).collect::<String>()) } else { text };
        Some(Snippet { line, text })
    }

    /// Readers of a shared index: the other PC's latest meaning passages.
    pub(super) fn reload_sense(&self) {
        for index in lock(&self.sense_open).values() {
            let _ = index.reader.reload();
        }
    }
}

/// A document of the main index worth understanding (source code, even
/// inside an archive, is left out).
fn eligible_doc(path: &str) -> bool {
    let (disk, inner) = split_inner(path);
    match inner {
        Some(inner) => FileKind::from_path(std::path::Path::new(inner_name(inner))) != Some(FileKind::Code),
        None => eligible_file(disk),
    }
}
