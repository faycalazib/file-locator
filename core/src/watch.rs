//! Live watching of a site's folders (goal.md §4). The paths changed on disk
//! are grouped until the disk is quiet (a save often writes a file several
//! times, copying a folder makes thousands of events), then handed over in
//! one batch.
//!
//! No file-id cache: it would walk every watched folder when the watch starts
//! (minutes for 100 000 files on a USB hard disk). Renames are simply seen as
//! "old path gone" + "new path created".

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use notify_debouncer_full::notify::{Config, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{new_debouncer_opt, DebounceEventResult, Debouncer, NoCache};

/// One batch of changes seen on disk.
#[derive(Debug, Default)]
pub struct Changes {
    /// Created, changed, removed or renamed paths (files or folders).
    pub paths: Vec<PathBuf>,
    /// Events were lost (overflow, watcher error): compare the whole site.
    pub rescan: bool,
}

impl Changes {
    pub fn merge(&mut self, other: Changes) {
        self.rescan |= other.rescan;
        let mut paths: BTreeSet<PathBuf> = std::mem::take(&mut self.paths).into_iter().collect();
        paths.extend(other.paths);
        self.paths = paths.into_iter().collect();
    }
}

/// Stops watching when dropped.
pub struct FolderWatcher {
    _debouncer: Debouncer<RecommendedWatcher, NoCache>,
}

impl FolderWatcher {
    /// Watches `roots` recursively; `on_changes` is called on the watcher's
    /// thread after `quiet` without events. `None` if no root can be watched.
    pub fn start(roots: &[PathBuf], quiet: Duration, on_changes: impl Fn(Changes) + Send + 'static) -> Option<Self> {
        let handler = move |result: DebounceEventResult| {
            let changes = match result {
                Ok(events) => {
                    let mut changes = Changes::default();
                    let mut paths = BTreeSet::new();
                    for event in events {
                        changes.rescan |= event.need_rescan();
                        if !event.kind.is_access() {
                            paths.extend(event.event.paths);
                        }
                    }
                    changes.paths = paths.into_iter().collect();
                    changes
                }
                Err(_) => Changes { paths: Vec::new(), rescan: true },
            };
            if changes.rescan || !changes.paths.is_empty() {
                on_changes(changes);
            }
        };
        let mut debouncer =
            new_debouncer_opt::<_, RecommendedWatcher, NoCache>(quiet, None, handler, NoCache, Config::default()).ok()?;
        let watched = roots.iter().filter(|root| debouncer.watch(root.as_path(), RecursiveMode::Recursive).is_ok()).count();
        (watched > 0).then_some(Self { _debouncer: debouncer })
    }
}
