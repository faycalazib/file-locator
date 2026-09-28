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

/// Watches every indexed site, then brings each one up to date, one after
/// the other (one disk at a time).
pub fn start_all(app: AppHandle) {
    std::thread::spawn(move || {
        let sites: Vec<SiteRecord> =
            app.state::<AppState>().engine().sites().into_iter().filter(|s| s.last_indexed.is_some()).collect();
        for site in &sites {
            watch(&app, site);
        }
        for site in &sites {
            run_indexing(&app, &site.id, &site.excluded, true);
        }
    });
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
