//! Facade used by the Tauri layer: dig sites, indexing, search, preview.
//!
//! Data layout (all under the user-chosen data folder, AppData by default):
//! ```text
//! <data>/sites.json          catalog of dig sites
//! <data>/saved-searches.json saved searches
//! <data>/indexes/<site id>/  Tantivy index of each site
//! <data>/indexes/<site id>.manifest.json  what was indexed, file by file
//! ```

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tantivy::{IndexWriter, TantivyDocument, Term};

use crate::crawler::{crawl, file_entry, is_reachable, Exclusions, FileEntry};
use crate::error::{CoreError, Result};
use crate::fsutil::write_atomic;
use crate::extract::{extract_one, split_inner, ExtractedDoc, SkipReason, INNER_SEP};
use crate::pipeline::process_files;
use crate::kind::FileKind;
use crate::lang::DocLang;
use crate::highlight::{preview_lines, PreviewLine};
use crate::index::{document, DocParts, SiteIndex};
use crate::manifest::{is_within, FileStamp, Manifest};
use crate::groups::{SiteGroup, SiteGroups};
use crate::saved::{Alert, SavedSearch, SavedSearches};
use crate::lang::detect;
use crate::scan::{find_folders, ScanTarget};
use crate::dupes::{self, exact_groups, similar_groups, DupFile, DupOptions, DupPhase, DupProgress, DupReport};
use crate::facets::{count_site, Facets};
use crate::query::ParsedQuery;
use crate::report::{terms as report_terms, DocRef, KeywordReport, KeywordRow, TermTotal};
use crate::scope::{inside, restrict_targets, site_scope, SiteScope};
use crate::search::{search_site, stored_body, DetectionTotals, Hit, SearchRequest};

mod meaning;
pub use meaning::{Embed, SenseHit, SensePlan, SenseProgress};

/// Index writer: memory per indexing thread (Tantivy needs ≥ 15 MB) and
/// maximum number of threads.
const WRITER_HEAP_PER_THREAD: usize = 64 * 1024 * 1024;
const MAX_WRITER_THREADS: usize = 8;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteRecord {
    pub id: String,
    /// Chosen by the user: data, never translated.
    pub name: String,
    pub roots: Vec<String>,
    pub doc_count: u64,
    pub size_bytes: u64,
    /// Unix seconds; `None` until the first indexing.
    pub last_indexed: Option<u64>,
    /// Files that could not be read at the last indexing, by reason.
    #[serde(default)]
    pub skipped: BTreeMap<String, u64>,
    /// Roots missing at the last indexing.
    #[serde(default)]
    pub missing_roots: Vec<String>,
    /// Folders excluded at the last indexing (reused by the folder watcher).
    #[serde(default)]
    pub excluded: Vec<String>,
    /// Meaning index turned on (Étape 8).
    #[serde(default)]
    pub sense: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IndexPhase {
    Scanning,
    Reading,
    Saving,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub site_id: String,
    pub phase: IndexPhase,
    /// Files found so far (scanning) / total to read.
    pub total: usize,
    pub done: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub hits: Vec<Hit>,
    pub total_files: usize,
    /// Returned files with at least one exact match.
    pub exact_files: usize,
    pub took_ms: u64,
    /// Detectors (lot 6.2): per kind of data, over every file found, not
    /// only the ones returned.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub detections: DetectionTotals,
    /// Counts per kind, language, year and site, when asked (lot 6.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facets: Option<Facets>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewDoc {
    pub path: String,
    /// `code` (mono, as is), `sheet` (Excel: tables), `slides` (PowerPoint:
    /// cards) or `prose`.
    pub layout: &'static str,
    pub lines: Vec<PreviewLine>,
    pub match_count: usize,
    /// Only part of the document is shown (long document).
    pub truncated: bool,
}

/// This PC's files, kept out of a shared data folder (lot 7.3).
const PERSONAL_FILES: [&str; 2] = ["saved-searches.json", "site-groups.json"];

pub struct Engine {
    data_dir: PathBuf,
    /// Saved searches, alerts, groups: this PC's, even when the data folder
    /// is shared (lot 7.3).
    personal_dir: PathBuf,
    /// Shared index (lot 7.3): `Some(pc)` while another PC keeps it up to
    /// date; this one then only reads.
    reader_of: Mutex<Option<String>>,
    /// Date of the catalog last read (readers see the other PC's changes).
    catalog_seen: Mutex<Option<std::time::SystemTime>>,
    sites: Mutex<Vec<SiteRecord>>,
    open: Mutex<HashMap<String, Arc<SiteIndex>>>,
    busy: Mutex<HashSet<String>>,
    saved: SavedSearches,
    groups: SiteGroups,
    /// Files changed / removed by the updates of each site, until the alerts
    /// look at them (lot 6.1).
    changes: Mutex<HashMap<String, SiteChanges>>,
    /// Meaning indexes opened (Étape 8).
    sense_open: Mutex<HashMap<String, Arc<crate::sense::store::SenseIndex>>>,
}

/// A word of the search found on an image (lot 6.6): its box as fractions
/// of the image turned like the photo.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Found through typo tolerance.
    pub fuzzy: bool,
}

/// Changed files, removed files (paths on disk).
type SiteChanges = (BTreeSet<String>, BTreeSet<String>);

/// Files an alert announces (lot 6.1).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertNews {
    pub saved_id: String,
    pub label: String,
    /// Documents that started matching.
    pub found: Vec<String>,
}

/// Most documents an alert remembers or checks in one go.
const ALERT_LIMIT: usize = 20_000;
/// Beyond this many changed files, the alert checks the whole site (same
/// answer, cheaper than a huge term set).
const ALERT_FILES: usize = 10_000;

/// Tantivy document of one extracted document of `file`. A document inside
/// an archive or a mailbox has its own size and date when the container
/// knows them, and the container's creation date.
fn build_document(f: &crate::index::Fields, path: &str, name: &str, doc: &ExtractedDoc, file: &FileEntry) -> TantivyDocument {
    document(
        f,
        &DocParts {
            path,
            file: &file.path.to_string_lossy(),
            name,
            inner: doc.inner.is_some(),
            kind: doc.kind.as_str(),
            lang: detect(&doc.text),
            text: &doc.text,
            size: doc.size.unwrap_or(file.size),
            modified: doc.modified.unwrap_or(file.modified),
            created: file.facts.created,
        },
    )
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Engine {
    /// Everything in one folder (tests, and a data folder of one's own).
    pub fn open(data_dir: &Path) -> Result<Self> {
        Self::open_with(data_dir, data_dir)
    }

    /// Catalog and indexes in `data_dir`, possibly shared; saved searches,
    /// alerts and groups in `personal_dir`, this PC's (lot 7.3). Those found
    /// in the data folder by an older version are moved there once.
    pub fn open_with(data_dir: &Path, personal_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir.join("indexes"))?;
        if personal_dir != data_dir {
            std::fs::create_dir_all(personal_dir)?;
            for name in PERSONAL_FILES {
                let (from, to) = (data_dir.join(name), personal_dir.join(name));
                if from.exists() && !to.exists() && std::fs::rename(&from, &to).is_err() {
                    // Another volume (a network share): copy, then remove.
                    std::fs::copy(&from, &to)?;
                    let _ = std::fs::remove_file(&from);
                }
            }
        }
        let catalog = data_dir.join("sites.json");
        let sites = if catalog.exists() { serde_json::from_slice(&std::fs::read(&catalog)?)? } else { Vec::new() };
        let seen = std::fs::metadata(&catalog).and_then(|m| m.modified()).ok();
        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            personal_dir: personal_dir.to_path_buf(),
            reader_of: Mutex::new(None),
            catalog_seen: Mutex::new(seen),
            sites: Mutex::new(sites),
            open: Mutex::new(HashMap::new()),
            busy: Mutex::new(HashSet::new()),
            saved: SavedSearches::open(personal_dir)?,
            groups: SiteGroups::open(personal_dir)?,
            changes: Mutex::new(HashMap::new()),
            sense_open: Mutex::new(HashMap::new()),
        })
    }

    pub fn personal_dir(&self) -> &Path {
        &self.personal_dir
    }

    /// Shared index (lot 7.3): `Some(pc)` = another PC keeps it up to date,
    /// this one only reads; `None` = this one does.
    pub fn set_reader_of(&self, pc: Option<String>) {
        let reading = pc.is_some();
        *lock(&self.reader_of) = pc;
        // Indexes opened for writing are opened again read-only, and back.
        lock(&self.open).clear();
        lock(&self.sense_open).clear();
        if reading {
            self.refresh_shared();
        }
    }

    pub fn reader_of(&self) -> Option<String> {
        lock(&self.reader_of).clone()
    }

    /// Refuses a change while another PC keeps the index up to date.
    fn writable(&self) -> Result<()> {
        match self.reader_of() {
            Some(pc) => Err(CoreError::ReadOnly { pc }),
            None => Ok(()),
        }
    }

    /// Reader of a shared index: sees what the other PC did — its catalog
    /// if it changed, and the last commit of each open index. Returns
    /// whether the catalog changed. Nothing to do for the holder.
    pub fn refresh_shared(&self) -> bool {
        if self.reader_of().is_none() {
            return false;
        }
        for index in lock(&self.open).values() {
            let _ = index.reader.reload();
        }
        self.reload_sense();
        let catalog = self.data_dir.join("sites.json");
        let modified = std::fs::metadata(&catalog).and_then(|m| m.modified()).ok();
        let mut seen = lock(&self.catalog_seen);
        if modified == *seen {
            return false;
        }
        let Some(sites) = std::fs::read(&catalog).ok().and_then(|b| serde_json::from_slice::<Vec<SiteRecord>>(&b).ok()) else {
            return false;
        };
        *seen = modified;
        drop(seen);
        // Sites removed over there are closed here.
        lock(&self.open).retain(|id, _| sites.iter().any(|s| s.id == *id));
        *lock(&self.sites) = sites;
        true
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn saved_searches(&self) -> Vec<SavedSearch> {
        self.saved.list()
    }

    pub fn save_search(&self, label: &str, query: &str, settings: serde_json::Value) -> Result<SavedSearch> {
        self.saved.save(label, query, settings)
    }

    /// Turns the alert of a saved search on: `request` (built by the UI) runs
    /// once, and what it finds already is remembered, never announced.
    pub fn enable_alert(&self, id: &str, request: SearchRequest, site_ids: Vec<String>) -> Result<SavedSearch> {
        let req = SearchRequest { limit: Some(ALERT_LIMIT), in_files: Vec::new(), ..request.clone() };
        let known: Vec<String> = site_ids.iter().filter(|s| self.site(s).is_ok()).cloned().collect();
        let seen = self.search(&known, &req)?.hits.into_iter().map(|h| h.path).collect();
        self.saved.set_alert(id, Some(Alert { request, site_ids: known, seen, fresh: Vec::new() }))
    }

    pub fn disable_alert(&self, id: &str) -> Result<SavedSearch> {
        self.saved.set_alert(id, None)
    }

    /// The user looked at an alert's results.
    pub fn mark_alert_read(&self, id: &str) -> Result<SavedSearch> {
        self.saved.mark_read(id)
    }

    /// After an update of `site_id`: the documents of the changed files that
    /// start matching an alert on this site, per alert (announced once).
    pub fn check_alerts(&self, site_id: &str) -> Result<Vec<AlertNews>> {
        let (changed, removed) = lock(&self.changes).remove(site_id).unwrap_or_default();
        if changed.is_empty() && removed.is_empty() {
            return Ok(Vec::new());
        }
        let changed: Vec<String> = changed.into_iter().collect();
        let removed: Vec<String> = removed.into_iter().collect();
        let mut news = Vec::new();
        for saved in self.saved.alerts_on(site_id) {
            let Some(alert) = &saved.alert else { continue };
            let found: Vec<String> = if changed.is_empty() {
                Vec::new()
            } else {
                let in_files = if changed.len() <= ALERT_FILES { changed.clone() } else { Vec::new() };
                let req = SearchRequest { limit: Some(ALERT_LIMIT), in_files, ..alert.request.clone() };
                self.search(&[site_id.to_owned()], &req)?.hits.into_iter().map(|h| h.path).collect()
            };
            let new = self.saved.record(&saved.id, &found, &removed)?;
            if !new.is_empty() {
                news.push(AlertNews { saved_id: saved.id.clone(), label: saved.label.clone(), found: new });
            }
        }
        Ok(news)
    }

    /// Site groups (lot 7.2), in creation order (Ctrl+1…9).
    pub fn site_groups(&self) -> Vec<SiteGroup> {
        self.groups.list()
    }

    /// A new group of these sites (unknown ids are left out).
    pub fn create_site_group(&self, name: &str, site_ids: Vec<String>) -> Result<SiteGroup> {
        self.groups.create(name, self.known_sites(site_ids))
    }

    pub fn update_site_group(&self, id: &str, name: Option<&str>, site_ids: Option<Vec<String>>) -> Result<SiteGroup> {
        let site_ids = site_ids.map(|ids| self.known_sites(ids));
        self.groups.update(id, name, site_ids)
    }

    pub fn remove_site_group(&self, id: &str) -> Result<()> {
        self.groups.remove(id)
    }

    fn known_sites(&self, ids: Vec<String>) -> Vec<String> {
        let sites = lock(&self.sites);
        ids.into_iter().filter(|id| sites.iter().any(|s| s.id == *id)).collect()
    }

    pub fn remove_saved_search(&self, id: &str) -> Result<()> {
        self.saved.remove(id)
    }

    fn save(&self, sites: &[SiteRecord]) -> Result<()> {
        Ok(write_atomic(&self.data_dir.join("sites.json"), &serde_json::to_vec_pretty(sites)?)?)
    }

    fn index_dir(&self, id: &str) -> PathBuf {
        self.data_dir.join("indexes").join(id)
    }

    fn site_index(&self, id: &str) -> Result<Arc<SiteIndex>> {
        let mut open = lock(&self.open);
        if let Some(index) = open.get(id) {
            return Ok(index.clone());
        }
        // Another PC's index is only read (nothing created or migrated).
        let index = Arc::new(if self.reader_of().is_some() {
            SiteIndex::open_existing(&self.index_dir(id))?
        } else {
            SiteIndex::open_or_create(&self.index_dir(id))?
        });
        open.insert(id.to_owned(), index.clone());
        Ok(index)
    }

    pub fn sites(&self) -> Vec<SiteRecord> {
        // A reader of a shared index lists what the other PC has now.
        self.refresh_shared();
        lock(&self.sites).clone()
    }

    pub fn site(&self, id: &str) -> Result<SiteRecord> {
        lock(&self.sites).iter().find(|s| s.id == id).cloned().ok_or_else(|| CoreError::SiteNotFound { id: id.to_owned() })
    }

    pub fn is_busy(&self, id: &str) -> bool {
        lock(&self.busy).contains(id)
    }

    pub fn add_site(&self, name: &str, roots: Vec<String>) -> Result<SiteRecord> {
        self.writable()?;
        let mut sites = lock(&self.sites);
        let base: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .chars()
            .take(24)
            .collect();
        let base = if base.is_empty() { "site".to_owned() } else { base };
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis());
        let record = SiteRecord {
            id: format!("{base}-{stamp:x}"),
            name: name.to_owned(),
            roots,
            doc_count: 0,
            size_bytes: 0,
            last_indexed: None,
            skipped: BTreeMap::new(),
            missing_roots: Vec::new(),
            sense: false,
            excluded: Vec::new(),
        };
        sites.push(record.clone());
        self.save(&sites)?;
        Ok(record)
    }

    /// Removes a site and deletes its index.
    pub fn remove_site(&self, id: &str) -> Result<()> {
        self.writable()?;
        if self.is_busy(id) {
            return Err(CoreError::IndexBusy { id: id.to_owned() });
        }
        let mut sites = lock(&self.sites);
        let before = sites.len();
        sites.retain(|s| s.id != id);
        if sites.len() == before {
            return Err(CoreError::SiteNotFound { id: id.to_owned() });
        }
        self.save(&sites)?;
        drop(sites);
        self.groups.forget_site(id)?;
        self.forget_sense(id)?;
        lock(&self.open).remove(id);
        let dir = self.index_dir(id);
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
        let manifest = self.manifest_path(id);
        if manifest.exists() {
            std::fs::remove_file(manifest)?;
        }
        Ok(())
    }

    /// Brings a site's index up to date with its folders. The first time (or
    /// after a format change) every file is read; afterwards only the files
    /// added, changed or removed since the last update (goal.md §4).
    /// `excluded` is remembered for the folder watcher.
    pub fn index_site(
        &self,
        id: &str,
        excluded: &[String],
        cancel: &AtomicBool,
        progress: &(dyn Fn(IndexProgress) + Sync),
    ) -> Result<SiteRecord> {
        self.writable()?;
        let site = self.site(id)?;
        self.exclusive(id, || self.index_site_inner(&site, excluded, cancel, progress))
    }

    /// Updates the index for paths reported by the folder watcher: files
    /// created or changed are read again, removed files and folders leave the
    /// index. Returns the updated site, or `None` when nothing changed.
    pub fn update_paths(
        &self,
        id: &str,
        paths: &[PathBuf],
        cancel: &AtomicBool,
        progress: &(dyn Fn(IndexProgress) + Sync),
    ) -> Result<Option<SiteRecord>> {
        self.writable()?;
        let site = self.site(id)?;
        self.exclusive(id, || {
            let Some(mut manifest) = self.manifest(&site) else {
                // Never indexed with this version: a full update instead.
                return self.index_site_inner(&site, &site.excluded, cancel, progress).map(Some);
            };
            let exclusions = self.exclusions(&site.excluded);
            let roots: Vec<PathBuf> = site.roots.iter().map(PathBuf::from).collect();
            let mut changed: BTreeMap<PathBuf, FileEntry> = BTreeMap::new();
            let mut removed: BTreeSet<String> = BTreeSet::new();
            for path in paths {
                let Some(root) = roots.iter().find(|root| path.starts_with(root)) else {
                    continue;
                };
                let key = path.to_string_lossy().into_owned();
                match std::fs::metadata(path) {
                    Ok(meta) if meta.is_dir() => {
                        // New or renamed folder: compare its whole content.
                        let files = if is_reachable(path, root, &exclusions) {
                            crawl(std::slice::from_ref(path), &exclusions, cancel, &|_| {}).files
                        } else {
                            Vec::new()
                        };
                        let seen: HashSet<String> = files.iter().map(|f| f.path.to_string_lossy().into_owned()).collect();
                        removed.extend(manifest.under(&key).filter(|p| !seen.contains(*p)).cloned());
                        for file in files {
                            let key = file.path.to_string_lossy();
                            if manifest.files.get(&*key).is_none_or(|stamp| !stamp.matches(&file)) {
                                changed.insert(file.path.clone(), file);
                            }
                        }
                    }
                    // A reported file is always read again: two saves within
                    // the same second keep the same size and date.
                    Ok(_) => match file_entry(path, root, &exclusions) {
                        Some(file) => {
                            changed.insert(path.clone(), file);
                        }
                        None if manifest.files.contains_key(&key) => {
                            removed.insert(key);
                        }
                        None => {}
                    },
                    Err(_) => removed.extend(manifest.under(&key).cloned()),
                }
            }
            if changed.is_empty() && removed.is_empty() {
                return Ok(None);
            }
            let removed: Vec<String> = removed.into_iter().collect();
            let changed = changed.into_values().collect();
            if !self.apply(&site.id, &mut manifest, false, changed, &removed, cancel, progress)? {
                return Ok(None);
            }
            self.update_site(&site.id, |s| {
                s.doc_count = manifest.doc_count();
                s.size_bytes = manifest.indexed_bytes();
                s.skipped = manifest.skipped();
                s.last_indexed = Some(now());
            })
            .map(Some)
        })
    }

    /// Runs `work` with the site marked busy (one indexing at a time per site).
    fn exclusive<T>(&self, id: &str, work: impl FnOnce() -> Result<T>) -> Result<T> {
        if !lock(&self.busy).insert(id.to_owned()) {
            return Err(CoreError::IndexBusy { id: id.to_owned() });
        }
        let result = work();
        lock(&self.busy).remove(id);
        result
    }

    /// The user's exclusions plus the data folder itself (it may sit inside a
    /// site: the index must never index itself).
    fn exclusions(&self, excluded: &[String]) -> Exclusions {
        Exclusions::new(&self.exclusions_list(excluded))
    }

    fn exclusions_list(&self, excluded: &[String]) -> Vec<String> {
        let mut patterns = excluded.to_vec();
        patterns.push(self.data_dir.to_string_lossy().into_owned());
        patterns
    }

    fn manifest_path(&self, id: &str) -> PathBuf {
        self.data_dir.join("indexes").join(format!("{id}.manifest.json"))
    }

    /// The site's manifest, or `None` when the index must be rebuilt.
    fn manifest(&self, site: &SiteRecord) -> Option<Manifest> {
        site.last_indexed.and_then(|_| Manifest::load(&self.manifest_path(&site.id)))
    }

    fn index_site_inner(
        &self,
        site: &SiteRecord,
        excluded: &[String],
        cancel: &AtomicBool,
        progress: &(dyn Fn(IndexProgress) + Sync),
    ) -> Result<SiteRecord> {
        let roots: Vec<PathBuf> = site.roots.iter().map(PathBuf::from).collect();
        let outcome = crawl(&roots, &self.exclusions(excluded), cancel, &|n| {
            progress(IndexProgress { site_id: site.id.clone(), phase: IndexPhase::Scanning, total: n, done: 0 })
        });
        if outcome.files.is_empty() && !outcome.missing_roots.is_empty() {
            let path = outcome.missing_roots[0].to_string_lossy().into_owned();
            self.update_site(&site.id, |s| s.missing_roots = vec![path.clone()])?;
            return Err(CoreError::RootUnavailable { path });
        }
        if cancel.load(Ordering::Relaxed) {
            return self.site(&site.id);
        }

        let missing: Vec<String> = outcome.missing_roots.iter().map(|p| p.to_string_lossy().into_owned()).collect();
        let (mut manifest, full) = match self.manifest(site) {
            Some(manifest) => (manifest, false),
            None => (Manifest::default(), true),
        };
        let crawled: HashSet<String> = outcome.files.iter().map(|f| f.path.to_string_lossy().into_owned()).collect();
        // The documents of a missing root (unplugged disk) are kept.
        let removed: Vec<String> = manifest
            .files
            .keys()
            .filter(|p| !crawled.contains(*p) && !missing.iter().any(|root| is_within(p, root)))
            .cloned()
            .collect();
        let changed: Vec<FileEntry> = outcome
            .files
            .into_iter()
            .filter(|f| manifest.files.get(&*f.path.to_string_lossy()).is_none_or(|stamp| !stamp.matches(f)))
            .collect();

        if !self.apply(&site.id, &mut manifest, full, changed, &removed, cancel, progress)? {
            return self.site(&site.id);
        }
        self.update_site(&site.id, |s| {
            s.doc_count = manifest.doc_count();
            s.size_bytes = manifest.indexed_bytes();
            s.last_indexed = Some(now());
            s.skipped = manifest.skipped();
            s.missing_roots = missing.clone();
            s.excluded = excluded.to_vec();
        })
    }

    /// Writes one batch of changes: the documents of `removed` files are
    /// deleted and the `changed` files are read again (`full`: the index is
    /// emptied first). Returns `false` if cancelled: nothing is written.
    #[allow(clippy::too_many_arguments)]
    fn apply(
        &self,
        site_id: &str,
        manifest: &mut Manifest,
        full: bool,
        mut changed: Vec<FileEntry>,
        removed: &[String],
        cancel: &AtomicBool,
        progress: &(dyn Fn(IndexProgress) + Sync),
    ) -> Result<bool> {
        let report = |phase, total, done| progress(IndexProgress { site_id: site_id.to_owned(), phase, total, done });
        let total = changed.len();
        report(IndexPhase::Reading, total, 0);
        if !full && changed.is_empty() && removed.is_empty() {
            return Ok(true);
        }

        let index = self.site_index(site_id)?;
        let f = index.fields;
        // One indexing thread per ~1000 files (max 8): each thread writes its own
        // segment, and a small batch must not pay for merging a dozen of them.
        let threads = (total / 1000).clamp(1, MAX_WRITER_THREADS);
        let mut writer: IndexWriter<TantivyDocument> =
            index.index.writer_with_num_threads(threads, threads * WRITER_HEAP_PER_THREAD)?;
        if full {
            writer.delete_all_documents()?;
            manifest.files.clear();
        }
        // Deletes first: a delete only affects the documents added before it.
        // Changed files are deleted too (their old version, if any).
        for path in removed {
            writer.delete_term(Term::from_field_text(f.file, path));
        }
        for file in &changed {
            writer.delete_term(Term::from_field_text(f.file, &file.path.to_string_lossy()));
        }

        let done = AtomicUsize::new(0);
        let stamps: Mutex<Vec<(String, FileStamp)>> = Mutex::new(Vec::with_capacity(total));
        process_files(&mut changed, cancel, |file, extracted| {
            let file_path = file.path.to_string_lossy();
            let (docs, skipped) = match extracted {
                Ok(docs) => {
                    let file_name = file.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    let mut added = 0;
                    for extracted in docs {
                        let (path, name) = match &extracted.inner {
                            Some(inner) => (
                                format!("{file_path}{INNER_SEP}{inner}"),
                                inner.rsplit([' ', '/', '›']).next().unwrap_or(inner).to_owned(),
                            ),
                            None => (file_path.clone().into_owned(), file_name.clone()),
                        };
                        let document = build_document(&f, &path, &name, &extracted, file);
                        if writer.add_document(document).is_ok() {
                            added += 1;
                        }
                    }
                    (added, None)
                }
                // Formats that are simply not supported yet are not failures.
                Err(SkipReason::Unsupported) => (0, None),
                Err(reason) => (0, serde_json::to_value(reason).ok().and_then(|v| v.as_str().map(str::to_owned))),
            };
            let stamp = FileStamp { size: file.size, modified: file.modified, docs, skipped, ocr: crate::extract::ocr::enabled() };
            lock(&stamps).push((file_path.into_owned(), stamp));
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n.is_multiple_of(64) || n == total {
                report(IndexPhase::Reading, total, n);
            }
        });

        if cancel.load(Ordering::Relaxed) {
            writer.rollback()?;
            return Ok(false);
        }
        report(IndexPhase::Saving, total, total);
        writer.commit()?;
        writer.wait_merging_threads()?;
        index.reader.reload()?;
        {
            let mut changes = lock(&self.changes);
            let (touched, gone) = changes.entry(site_id.to_owned()).or_default();
            touched.extend(changed.iter().map(|f| f.path.to_string_lossy().into_owned()));
            gone.extend(removed.iter().cloned());
        }

        for path in removed {
            manifest.files.remove(path);
        }
        manifest.files.extend(stamps.into_inner().unwrap_or_default());
        manifest.save(&self.manifest_path(site_id))?;
        Ok(true)
    }

    fn update_site(&self, id: &str, change: impl FnOnce(&mut SiteRecord)) -> Result<SiteRecord> {
        let mut sites = lock(&self.sites);
        let site = sites.iter_mut().find(|s| s.id == id).ok_or_else(|| CoreError::SiteNotFound { id: id.to_owned() })?;
        change(site);
        let updated = site.clone();
        self.save(&sites)?;
        Ok(updated)
    }

    /// Searches several sites at once (goal.md §3: multi-folder search).
    pub fn search(&self, site_ids: &[String], req: &SearchRequest) -> Result<SearchResponse> {
        let started = Instant::now();
        self.refresh_shared();
        let parsed = req.parsed();
        if req.folders {
            return self.search_folders(site_ids, req, started);
        }
        if req.is_empty() {
            return Ok(SearchResponse {
                hits: Vec::new(),
                total_files: 0,
                exact_files: 0,
                took_ms: 0,
                detections: DetectionTotals::new(),
                facets: None,
            });
        }
        let mut hits = Vec::new();
        let mut total = 0;
        let mut detections = DetectionTotals::new();
        let mut facets = req.facets.then(Facets::default);
        for id in site_ids {
            let site = self.site(id)?;
            if site.last_indexed.is_none() {
                continue;
            }
            let prefixes = match site_scope(&site.roots, req.folder()) {
                SiteScope::Whole => Vec::new(),
                SiteScope::Prefixes(prefixes) => prefixes,
                SiteScope::Outside => continue,
            };
            let answer = search_site(id, &*self.site_index(id)?, &parsed, req, &prefixes)?;
            hits.extend(answer.hits);
            total += answer.total;
            for (code, t) in answer.detections {
                let sum = detections.entry(code).or_default();
                sum.files += t.files;
                sum.matches += t.matches;
            }
            if let (Some(all), Some(site_facets)) = (facets.as_mut(), answer.facets) {
                all.merge(site_facets);
                all.sites.insert(id.clone(), answer.total);
            }
        }
        // Sites not searched: what they would bring (when the index answers alone).
        let checked = req.verified(&parsed) || req.disk_check()?.is_some();
        if let Some(all) = facets.as_mut().filter(|_| !checked) {
            for site in self.sites().into_iter().filter(|s| s.last_indexed.is_some() && !site_ids.contains(&s.id)) {
                let prefixes = match site_scope(&site.roots, req.folder()) {
                    SiteScope::Whole => Vec::new(),
                    SiteScope::Prefixes(prefixes) => prefixes,
                    SiteScope::Outside => continue,
                };
                all.sites.insert(site.id.clone(), count_site(&*self.site_index(&site.id)?, &parsed, req, &prefixes)?);
            }
        }
        hits.sort_by(|a, b| b.score.total_cmp(&a.score));
        hits.truncate(req.limit.unwrap_or(200));
        let exact_files = hits.iter().filter(|h| h.exact_count > 0).count();
        Ok(SearchResponse { hits, total_files: total, exact_files, took_ms: started.elapsed().as_millis() as u64, detections, facets })
    }

    /// Folders by name (lot 5.1). They are not indexed: the sites' folders are
    /// walked (folder entries only), with each site's own exclusions.
    fn search_folders(&self, site_ids: &[String], req: &SearchRequest, started: Instant) -> Result<SearchResponse> {
        let hits = Mutex::new(Vec::new());
        let never = AtomicBool::new(false);
        // Limited to a folder: one walk of that folder (scope.rs), with the
        // exclusions of the site holding it.
        if let Some(folder) = req.folder() {
            let targets: Vec<ScanTarget> = site_ids
                .iter()
                .filter_map(|id| self.site(id).ok())
                .map(|s| ScanTarget { site_id: s.id, roots: s.roots.iter().map(PathBuf::from).collect() })
                .collect();
            let restricted = restrict_targets(&targets, folder);
            let excluded = self.site(&restricted[0].site_id).map(|s| s.excluded).unwrap_or_default();
            find_folders(&restricted, &self.exclusions_list(&excluded), req, &never, &|hit| lock(&hits).push(hit))?;
        }
        for id in site_ids.iter().filter(|_| req.folder().is_none()) {
            let site = self.site(id)?;
            let target = ScanTarget { site_id: site.id.clone(), roots: site.roots.iter().map(PathBuf::from).collect() };
            let excluded = self.exclusions_list(&site.excluded);
            find_folders(std::slice::from_ref(&target), &excluded, req, &never, &|hit| lock(&hits).push(hit))?;
        }
        let mut hits = hits.into_inner().unwrap_or_default();
        hits.sort_by(|a, b| a.path.cmp(&b.path));
        hits.truncate(req.limit.unwrap_or(200));
        Ok(SearchResponse {
            total_files: hits.len(),
            exact_files: 0,
            hits,
            took_ms: started.elapsed().as_millis() as u64,
            detections: DetectionTotals::new(),
            facets: None,
        })
    }

    /// The indexed site that holds `folder` (Explorer right-click, lot 5.7):
    /// a search in that folder can then use the index.
    pub fn site_holding(&self, folder: &str) -> Option<SiteRecord> {
        self.sites()
            .into_iter()
            .find(|s| s.last_indexed.is_some() && s.roots.iter().any(|r| inside(r, folder).is_some()))
    }

    /// Duplicates in these sites (lot 6.5): identical files (from the
    /// manifests, hashed by size group) and near-identical documents (from
    /// their indexed text). Largest groups first.
    pub fn find_duplicates(
        &self,
        site_ids: &[String],
        options: DupOptions,
        cancel: &AtomicBool,
        progress: &(dyn Fn(DupProgress) + Sync),
    ) -> Result<DupReport> {
        let started = Instant::now();
        let sites: Vec<SiteRecord> = site_ids.iter().filter_map(|id| self.site(id).ok()).filter(|s| s.last_indexed.is_some()).collect();
        let mut report = DupReport::default();

        let mut exact_of: HashMap<String, usize> = HashMap::new();
        if options.exact {
            let mut seen = HashSet::new();
            let files: Vec<DupFile> = sites
                .iter()
                .filter_map(|site| self.manifest(site).map(|m| (site.id.clone(), m)))
                .flat_map(|(site_id, manifest)| {
                    manifest.files.into_iter().map(move |(path, stamp)| DupFile { site_id: site_id.clone(), path, size: stamp.size, modified: stamp.modified })
                })
                .filter(|f| seen.insert(f.path.clone()))
                .collect();
            let (groups, hashed) = exact_groups(files, cancel, &|done, total| progress(DupProgress { phase: DupPhase::Hashing, done, total }));
            for (i, group) in groups.iter().enumerate() {
                for file in &group.files {
                    exact_of.insert(file.path.clone(), i);
                }
            }
            report.files_hashed = hashed;
            report.groups.extend(groups);
        }

        if options.similar && !cancel.load(Ordering::Relaxed) {
            let mut docs = Vec::new();
            let mut seen = HashSet::new();
            for site in &sites {
                let index = self.site_index(&site.id)?;
                let f = index.fields;
                let searcher = index.reader.searcher();
                for (ordinal, segment) in searcher.segment_readers().iter().enumerate() {
                    for doc_id in segment.doc_ids_alive() {
                        let doc: TantivyDocument = searcher.doc(tantivy::DocAddress::new(ordinal as u32, doc_id))?;
                        let text = |field| doc.get_first(field).and_then(|v| tantivy::schema::Value::as_str(&v)).unwrap_or_default().to_owned();
                        let number = |field| doc.get_first(field).and_then(|v| tantivy::schema::Value::as_u64(&v)).unwrap_or_default();
                        let path = text(f.path);
                        let body = text(f.body);
                        if body.len() < dupes::MIN_CHARS || !seen.insert(path.clone()) {
                            continue;
                        }
                        docs.push((DupFile { site_id: site.id.clone(), path, size: number(f.size), modified: number(f.modified) }, body));
                    }
                }
            }
            // Identical files are already shown as such.
            let skip = |a: &DupFile, b: &DupFile| exact_of.get(&a.path).is_some_and(|g| exact_of.get(&b.path) == Some(g));
            let (groups, compared) = similar_groups(docs, options.threshold.clamp(0.5, 1.0), &skip, cancel, &|done, total| {
                progress(DupProgress { phase: DupPhase::Comparing, done, total })
            });
            report.docs_compared = compared;
            report.groups.extend(groups);
        }

        for group in &mut report.groups {
            group.files.sort_by(|a, b| a.path.cmp(&b.path));
        }
        report.groups.sort_by(|a, b| b.wasted.cmp(&a.wasted).then(b.files.len().cmp(&a.files.len())).then(a.files[0].path.cmp(&b.files[0].path)));
        report.cancelled = cancel.load(Ordering::Relaxed);
        report.took_ms = started.elapsed().as_millis() as u64;
        Ok(report)
    }

    /// The boxes of the words matching the search on an image (lot 6.6):
    /// the image is read again by the OCR, for the position of each word;
    /// the words are matched with the same rules as the highlighted text,
    /// line by line. An image inside an archive or an e-mail is extracted
    /// first (lot 6.7).
    pub fn image_matches(&self, path: &str, req: &SearchRequest) -> Result<Vec<ImageBox>> {
        let (file, inner) = split_inner(path);
        let matcher = req.matcher(&req.parsed())?;
        if matcher.is_empty() {
            return Ok(Vec::new());
        }
        let bytes = match inner {
            Some(_) => crate::unpack::read(path)?.1,
            None => std::fs::read(file)?,
        };
        let Ok(lines) = crate::extract::ocr::image_words(&bytes) else {
            return Ok(Vec::new());
        };
        let all: Vec<&str> = lines.iter().flatten().map(|w| w.text.as_str()).collect();
        let lang = detect(&all.join(" "));
        let mut boxes = Vec::new();
        for line in &lines {
            // The line's text, each word's range in it.
            let mut text = String::new();
            let mut spans = Vec::with_capacity(line.len());
            for word in line {
                if !text.is_empty() {
                    text.push(' ');
                }
                spans.push(text.len()..text.len() + word.text.len());
                text.push_str(&word.text);
            }
            for m in matcher.find(&text, lang) {
                let hit: Vec<&crate::extract::ocr::OcrWord> =
                    line.iter().zip(&spans).filter(|(_, s)| s.start < m.range.end && m.range.start < s.end).map(|(w, _)| w).collect();
                let Some(first) = hit.first() else { continue };
                let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x + first.w, first.y + first.h);
                for w in &hit[1..] {
                    x0 = x0.min(w.x);
                    y0 = y0.min(w.y);
                    x1 = x1.max(w.x + w.w);
                    y1 = y1.max(w.y + w.h);
                }
                boxes.push(ImageBox { x: x0, y: y0, w: x1 - x0, h: y1 - y0, fuzzy: m.fuzzy });
            }
        }
        Ok(boxes)
    }

    /// Keyword report (lot 6.4): occurrences of each term of the query in
    /// each document (its stored text, or the file read from disk for
    /// live-scan results), and the totals per term.
    pub fn keyword_report(&self, docs: &[DocRef], req: &SearchRequest) -> Result<KeywordReport> {
        let parsed = req.parsed();
        let terms = report_terms(&parsed);
        let matchers = terms
            .iter()
            .map(|(_, clause)| req.matcher(&ParsedQuery { must: vec![vec![clause.clone()]], must_not: Vec::new(), lines: parsed.lines }))
            .collect::<Result<Vec<_>>>()?;
        let mut totals = vec![TermTotal::default(); terms.len()];
        let mut rows = Vec::with_capacity(docs.len());
        for doc in docs {
            let stored = match self.site(&doc.site_id).and_then(|_| self.site_index(&doc.site_id)) {
                Ok(index) => stored_body(&index, &doc.path)?,
                Err(_) => None,
            };
            let Some((body, lang)) = stored.map(|(b, l, _)| (b, l)).or_else(|| read_from_disk(&doc.path).ok().map(|(b, l, _)| (b, l))) else {
                continue;
            };
            let counts: Vec<usize> = matchers.iter().map(|m| m.find(&body, lang).len()).collect();
            for (total, &n) in totals.iter_mut().zip(&counts) {
                if n > 0 {
                    total.files += 1;
                    total.matches += n;
                }
            }
            rows.push(KeywordRow { site_id: doc.site_id.clone(), path: doc.path.clone(), counts });
        }
        Ok(KeywordReport { terms: terms.into_iter().map(|(name, _)| name).collect(), rows, totals })
    }

    /// The text of one file with the request's matches marked: from the index,
    /// or read from disk (files found by a live scan, sites not indexed yet).
    pub fn preview(&self, site_id: &str, path: &str, req: &SearchRequest) -> Result<PreviewDoc> {
        self.preview_passage(site_id, path, req, None)
    }

    /// Same, with the passage found by meaning marked (lot 8.3): the preview
    /// opens on it when no word of the search is in the document.
    pub fn preview_passage(&self, site_id: &str, path: &str, req: &SearchRequest, passage: Option<(usize, usize)>) -> Result<PreviewDoc> {
        // Never open (= create) an index for an unknown site id.
        let stored = match self.site(site_id).and_then(|_| self.site_index(site_id)) {
            Ok(index) => stored_body(&index, path)?,
            Err(_) => None,
        };
        let (body, lang, kind) = match stored {
            Some(found) => found,
            None => read_from_disk(path)?,
        };
        let matcher = req.matcher(&req.parsed())?;
        let mut matches = if matcher.is_empty() { Vec::new() } else { matcher.find(&body, lang) };
        if let Some(range) = passage.filter(|_| matches.is_empty()).and_then(|(a, b)| meaning::passage_range(&body, a, b)) {
            matches.push(crate::highlight::Match { range, fuzzy: false });
        }
        // Sheets become tables and slides cards in the preview (rich preview).
        let layout = match kind.as_str() {
            "code" => "code",
            "excel" => "sheet",
            "powerpoint" => "slides",
            _ => "prose",
        };
        let (lines, truncated) = preview_lines(&body, &matches, matches!(layout, "sheet" | "slides"));
        Ok(PreviewDoc {
            path: path.to_owned(),
            layout,
            lines,
            match_count: matches.len(),
            truncated,
        })
    }
}

/// Reads one document directly from disk (inside a ZIP or PST if needed).
fn read_from_disk(path: &str) -> Result<(String, DocLang, String)> {
    let not_found = || CoreError::FileNotIndexed { path: path.to_owned() };
    let (file, inner) = split_inner(path);
    let kind = FileKind::from_path(Path::new(file)).ok_or_else(not_found)?;
    let doc = extract_one(Path::new(file), kind, inner).map_err(|_| not_found())?;
    let lang = detect(&doc.text);
    Ok((doc.text, lang, doc.kind.as_str().to_owned()))
}

/// Moves the whole data folder (catalog + indexes) to a new location.
/// If the target already holds a Prospector catalog, it is simply reused.
pub fn relocate_data(from: &Path, to: &Path) -> Result<()> {
    if to.join("sites.json").exists() || from == to || !from.join("sites.json").exists() {
        std::fs::create_dir_all(to)?;
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if to.exists() && std::fs::read_dir(to)?.next().is_some() {
        // Non-empty folder that is not a Prospector folder: use a subfolder.
        return relocate_data(from, &to.join("Prospector"));
    }
    if to.exists() {
        std::fs::remove_dir(to)?;
    }
    // Same volume: instant rename. Otherwise copy, then delete the old copy.
    if std::fs::rename(from, to).is_err() {
        copy_dir(from, to)?;
        std::fs::remove_dir_all(from)?;
    }
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
