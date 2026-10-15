//! Live folder watching (goal.md §4). Every indexed site has a watcher; its
//! batches of changes update the index in a worker thread, then
//! `index://finished` refreshes the rail and the current results.
//! At startup, each indexed site is first brought up to date: files changed
//! while Prospector was closed are read again.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Mutex;
use std::time::Duration;

use prospector_core::{Changes, CoreError, FolderWatcher, SiteRecord};
use tauri::{AppHandle, Emitter, Manager};

use crate::background;
use crate::commands::{run_indexing, view, IndexFinished};
use crate::state::AppState;

/// Quiet time before a batch of changes is indexed.
const QUIET: Duration = Duration::from_secs(2);
/// Wait before retrying while the site is being indexed.
const RETRY: Duration = Duration::from_secs(3);
/// Batches smaller than this update silently (no progress bar in the rail).
const PROGRESS_FROM: usize = 50;
/// A batch that fails (file briefly locked by an antivirus, BUG-026) is
/// retried; after that, the next update of the site catches it up.
const ATTEMPTS: u32 = 3;

#[derive(Default)]
pub struct Watchers(Mutex<HashMap<String, FolderWatcher>>);

impl Watchers {
    fn map(&self) -> std::sync::MutexGuard<'_, HashMap<String, FolderWatcher>> {
        self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn is_watching(&self, id: &str) -> bool {
        self.map().contains_key(id)
    }

    /// Dropping the watcher stops it; its worker thread then ends.
    pub fn unwatch(&self, id: &str) {
        self.map().remove(id);
    }

    pub fn unwatch_all(&self) {
        self.map().clear();
    }
}

/// Starts watching a site's folders (no-op if already watched).
pub fn watch(app: &AppHandle, site: &SiteRecord) {
    let state = app.state::<AppState>();
    if state.watchers.is_watching(&site.id) {
        return;
    }
    let roots: Vec<PathBuf> = site.roots.iter().map(PathBuf::from).collect();
    let (tx, rx) = channel::<Changes>();
    let tx = Mutex::new(tx);
    let Some(watcher) = FolderWatcher::start(&roots, QUIET, move |changes| {
        if let Ok(tx) = tx.lock() {
            let _ = tx.send(changes);
        }
    }) else {
        return;
    };
    state.watchers.map().insert(site.id.clone(), watcher);
    let app = app.clone();
    let id = site.id.clone();
    std::thread::spawn(move || worker(&app, &id, &rx));
}

/// Watches every indexed site, then brings each one up to date (BUG-072):
/// the sites of one disk one after the other (no seeking between two
/// folders), the disks side by side, the small sites first (a site with a
/// 2 GB archive to read again no longer holds the others for hours). A site
/// whose first indexing was cut off (app closed meanwhile) comes last on its
/// disk, then is watched (BUG-069).
pub fn start_all(app: AppHandle) {
    std::thread::spawn(move || {
        let sites = app.state::<AppState>().engine().sites();
        for site in sites.iter().filter(|s| s.last_indexed.is_some()) {
            watch(&app, site);
        }
        for disk in startup_plan(sites) {
            let app = app.clone();
            std::thread::spawn(move || {
                for site in &disk {
                    run_indexing(&app, &site.id, &site.excluded, true);
                }
            });
        }
    });
}

/// The sites grouped by disk, each group in the order it is brought up to
/// date: indexed sites from the smallest, then the unfinished ones.
fn startup_plan(sites: Vec<SiteRecord>) -> Vec<Vec<SiteRecord>> {
    let mut disks: Vec<(String, Vec<SiteRecord>)> = Vec::new();
    for site in sites {
        let key = disk_of(site.roots.first().map_or("", String::as_str));
        match disks.iter_mut().find(|(k, _)| *k == key) {
            Some((_, group)) => group.push(site),
            None => disks.push((key, vec![site])),
        }
    }
    disks
        .into_iter()
        .map(|(_, mut group)| {
            group.sort_by_key(|s| (s.last_indexed.is_none(), s.size_bytes));
            group
        })
        .collect()
}

/// The disk of a folder: `D:` for `d:\Docs`, `\\server\share` for a network
/// folder.
fn disk_of(root: &str) -> String {
    let unc = root.strip_prefix(r"\\").map(|rest| rest.split(['\\', '/']).take(2).collect::<Vec<_>>().join(r"\"));
    match unc {
        Some(share) => format!(r"\\{}", share.to_lowercase()),
        None => root.split(['\\', '/']).next().unwrap_or(root).to_uppercase(),
    }
}

/// Applies the batches of one site until its watcher is dropped.
fn worker(app: &AppHandle, id: &str, rx: &Receiver<Changes>) {
    while let Ok(mut changes) = rx.recv() {
        let mut failures = 0;
        loop {
            // Changes that arrived meanwhile join the batch.
            while let Ok(more) = rx.try_recv() {
                changes.merge(more);
            }
            let state = app.state::<AppState>();
            let engine = state.engine();
            if changes.rescan {
                let Ok(site) = engine.site(id) else { return };
                if engine.is_busy(id) {
                    std::thread::sleep(RETRY);
                    continue;
                }
                run_indexing(app, id, &site.excluded, true);
                break;
            }
            let large = changes.paths.len() >= PROGRESS_FROM;
            let result = engine.update_paths(id, &changes.paths, &AtomicBool::new(false), &|p| {
                if large || p.total >= PROGRESS_FROM {
                    let _ = app.emit("index://progress", p);
                }
            });
            match result {
                Err(CoreError::IndexBusy { .. }) => std::thread::sleep(RETRY),
                Err(CoreError::SiteNotFound { .. }) => return,
                Ok(Some(record)) => {
                    // The meaning index follows (Étape 8).
                    crate::sense::wake(app);
                    let site = Some(view(&state, record));
                    let _ = app.emit("index://finished", IndexFinished { site_id: id.to_owned(), site, error: None });
                    if let Ok(news) = engine.check_alerts(id) {
                        background::announce(app, news);
                    }
                    break;
                }
                Ok(None) => break,
                Err(_) => {
                    failures += 1;
                    if failures >= ATTEMPTS {
                        break;
                    }
                    std::thread::sleep(RETRY);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(name: &str, root: &str, size: u64, indexed: bool) -> SiteRecord {
        serde_json::from_value(serde_json::json!({
            "id": name, "name": name, "roots": [root], "docCount": 0, "sizeBytes": size,
            "lastIndexed": if indexed { Some(1) } else { None },
        }))
        .unwrap()
    }

    #[test]
    fn small_sites_first_one_disk_at_a_time() {
        let plan = startup_plan(vec![
            site("git", r"F:\GIT Repositories", 190_000_000, true),
            site("religion", r"H:\Religion", 3_800_000_000, true),
            site("new", r"h:\Nouveau", 10, false),
            site("youtube", r"H:\AI\Youtube", 69_000_000, true),
            site("nas", r"\\NAS\Docs\Contrats", 5, true),
            site("nas2", r"\\nas\docs\Factures", 1, true),
        ]);
        let names: Vec<Vec<&str>> = plan.iter().map(|g| g.iter().map(|s| s.name.as_str()).collect()).collect();
        // One group per disk (letter case and network share case ignored);
        // in each, the smallest first and the unfinished site last.
        assert_eq!(names, [vec!["git"], vec!["youtube", "religion", "new"], vec!["nas2", "nas"]]);
    }
}
