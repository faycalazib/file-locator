//! Recursive, parallel walk of a dig site's folders (goal.md §2 `crawler/`).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::{WalkBuilder, WalkState};

use crate::disk::Facts;
use crate::kind::FileKind;

/// Folders nobody wants to search, skipped whatever the site.
const DEFAULT_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".svn",
    ".hg",
    "node_modules",
    "__pycache__",
    ".venv",
    "target",
    "$recycle.bin",
    "system volume information",
];

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: PathBuf,
    pub kind: FileKind,
    pub size: u64,
    /// Unix seconds.
    pub modified: u64,
    /// Creation, last access, attributes (lot 5.8).
    pub facts: Facts,
}

#[derive(Debug, Default)]
pub struct CrawlOutcome {
    pub files: Vec<FileEntry>,
    /// Roots that do not exist or cannot be opened.
    pub missing_roots: Vec<PathBuf>,
}

/// User exclusions: absolute folders (`D:\Clients\_old`) or globs (`**\node_modules`).
pub struct Exclusions {
    prefixes: Vec<String>,
    globs: GlobSet,
}

impl Exclusions {
    pub fn new(patterns: &[String]) -> Self {
        let mut prefixes = Vec::new();
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            let normalized = normalize_path(pattern);
            if normalized.contains('*') || normalized.contains('?') {
                if let Ok(glob) = Glob::new(&normalized) {
                    builder.add(glob);
                }
            } else if !normalized.is_empty() {
                prefixes.push(normalized.trim_end_matches('/').to_owned());
            }
        }
        Self { prefixes, globs: builder.build().unwrap_or_else(|_| GlobSet::empty()) }
    }

    pub fn is_excluded(&self, path: &Path) -> bool {
        let normalized = normalize_path(&path.to_string_lossy());
        self.prefixes.iter().any(|p| normalized == *p || normalized.starts_with(&format!("{p}/")))
            || self.globs.is_match(&normalized)
    }
}

/// Lowercase, forward slashes: Windows paths compare case-insensitively.
fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

fn is_default_excluded(name: &str) -> bool {
    DEFAULT_EXCLUDED_DIRS.contains(&name.to_lowercase().as_str())
}

/// Lists every file of a recognized family under `roots`.
/// `on_found` is called with the running count (for the progress bar).
pub fn crawl(roots: &[PathBuf], exclusions: &Exclusions, cancel: &AtomicBool, on_found: &(dyn Fn(usize) + Sync)) -> CrawlOutcome {
    crawl_with(roots, exclusions, false, cancel, on_found)
}

/// [`crawl`], with hidden files and folders too when `hidden` (live scan
/// asked for hidden files, lot 5.8). The default-excluded folders stay out.
pub fn crawl_with(
    roots: &[PathBuf],
    exclusions: &Exclusions,
    hidden: bool,
    cancel: &AtomicBool,
    on_found: &(dyn Fn(usize) + Sync),
) -> CrawlOutcome {
    let mut outcome = CrawlOutcome::default();
    let existing: Vec<&PathBuf> = roots
        .iter()
        .filter(|root| {
            let ok = root.is_dir() || root.is_file();
            if !ok {
                outcome.missing_roots.push((*root).clone());
            }
            ok
        })
        .collect();
    let Some((first, rest)) = existing.split_first() else {
        return outcome;
    };

    let mut builder = WalkBuilder::new(first);
    for root in rest {
        builder.add(root);
    }
    builder
        .hidden(!hidden)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .ignore(false)
        .parents(false)
        .follow_links(false);

    let files = Mutex::new(Vec::new());
    let found = AtomicUsize::new(0);

    builder.build_parallel().run(|| {
        Box::new(|entry| {
            if cancel.load(Ordering::Relaxed) {
                return WalkState::Quit;
            }
            let Ok(entry) = entry else {
                return WalkState::Continue;
            };
            let path = entry.path();
            let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
            if is_dir {
                let name = entry.file_name().to_string_lossy();
                if entry.depth() > 0 && (is_default_excluded(&name) || exclusions.is_excluded(path)) {
                    return WalkState::Skip;
                }
                return WalkState::Continue;
            }
            if exclusions.is_excluded(path) {
                return WalkState::Continue;
            }
            let Some(kind) = FileKind::from_path(path) else {
                return WalkState::Continue;
            };
            let Ok(meta) = entry.metadata() else {
                return WalkState::Continue;
            };
            let entry = FileEntry {
                path: path.to_path_buf(),
                kind,
                size: meta.len(),
                modified: modified_secs(&meta),
                facts: Facts::from_meta(path, &meta),
            };
            if let Ok(mut list) = files.lock() {
                list.push(entry);
            }
            let n = found.fetch_add(1, Ordering::Relaxed) + 1;
            if n.is_multiple_of(256) {
                on_found(n);
            }
            WalkState::Continue
        })
    });

    outcome.files = files.into_inner().unwrap_or_default();
    on_found(outcome.files.len());
    outcome
}

/// Every folder under `roots` (the roots themselves excluded) whose name is
/// accepted by `keep`, with its modification date. Same rules as [`crawl`]:
/// hidden, default-excluded and user-excluded folders are skipped with their
/// content.
pub fn crawl_folders(
    roots: &[PathBuf],
    exclusions: &Exclusions,
    cancel: &AtomicBool,
    keep: &(dyn Fn(&str) -> bool + Sync),
) -> Vec<FileEntryDir> {
    let existing: Vec<&PathBuf> = roots.iter().filter(|root| root.is_dir()).collect();
    let Some((first, rest)) = existing.split_first() else {
        return Vec::new();
    };
    let mut builder = WalkBuilder::new(first);
    for root in rest {
        builder.add(root);
    }
    builder.hidden(true).git_ignore(false).git_global(false).git_exclude(false).ignore(false).parents(false).follow_links(false);

    let folders = Mutex::new(Vec::new());
    builder.build_parallel().run(|| {
        Box::new(|entry| {
            if cancel.load(Ordering::Relaxed) {
                return WalkState::Quit;
            }
            let Ok(entry) = entry else {
                return WalkState::Continue;
            };
            if !entry.file_type().is_some_and(|t| t.is_dir()) {
                return WalkState::Continue;
            }
            if entry.depth() == 0 {
                return WalkState::Continue;
            }
            let name = entry.file_name().to_string_lossy();
            if is_default_excluded(&name) || exclusions.is_excluded(entry.path()) {
                return WalkState::Skip;
            }
            if keep(&name) {
                let (modified, facts) =
                    entry.metadata().map_or((0, Facts::default()), |m| (modified_secs(&m), Facts::from_meta(entry.path(), &m)));
                if let Ok(mut list) = folders.lock() {
                    list.push(FileEntryDir { path: entry.path().to_path_buf(), modified, facts });
                }
            }
            WalkState::Continue
        })
    });
    let mut folders = folders.into_inner().unwrap_or_default();
    folders.sort_by(|a, b| a.path.cmp(&b.path));
    folders
}

/// A folder found by [`crawl_folders`].
#[derive(Clone, Debug)]
pub struct FileEntryDir {
    pub path: PathBuf,
    /// Unix seconds.
    pub modified: u64,
    pub facts: Facts,
}

fn modified_secs(meta: &std::fs::Metadata) -> u64 {
    meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs())
}

/// Hidden like the crawl sees it: dot name, or the Windows "hidden" attribute.
fn is_hidden(path: &Path) -> bool {
    if path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        if let Ok(meta) = std::fs::symlink_metadata(path) {
            return meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0;
        }
    }
    false
}

/// Would a crawl from `root` reach `path`? (no hidden or excluded folder on
/// the way, `path` itself included.) Used for the paths reported by the
/// folder watcher.
pub fn is_reachable(path: &Path, root: &Path, exclusions: &Exclusions) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let mut current = root.to_path_buf();
    for part in relative.components() {
        current.push(part);
        let name = part.as_os_str().to_string_lossy();
        if is_default_excluded(&name) || exclusions.is_excluded(&current) || is_hidden(&current) {
            return false;
        }
    }
    true
}

/// The entry of one file as a crawl from `root` would list it; `None` if the
/// crawl would skip it (hidden, excluded, unknown family, not a file).
pub fn file_entry(path: &Path, root: &Path, exclusions: &Exclusions) -> Option<FileEntry> {
    let kind = FileKind::from_path(path)?;
    let meta = std::fs::metadata(path).ok().filter(std::fs::Metadata::is_file)?;
    is_reachable(path, root, exclusions).then(|| FileEntry {
        path: path.to_path_buf(),
        kind,
        size: meta.len(),
        modified: modified_secs(&meta),
        facts: Facts::from_meta(path, &meta),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusions_by_prefix_and_glob() {
        let ex = Exclusions::new(&[r"D:\Clients\_corbeille".into(), r"**\node_modules".into(), "**/*.tmp".into()]);
        assert!(ex.is_excluded(Path::new(r"d:\clients\_CORBEILLE")));
        assert!(ex.is_excluded(Path::new(r"D:\Clients\_corbeille\x.pdf")));
        assert!(!ex.is_excluded(Path::new(r"D:\Clients\_corbeille2\x.pdf")));
        assert!(ex.is_excluded(Path::new(r"H:\dev\app\node_modules")));
        assert!(ex.is_excluded(Path::new(r"C:\a\b.tmp")));
        assert!(!ex.is_excluded(Path::new(r"C:\a\b.txt")));
    }

    #[test]
    fn reachability_matches_the_crawl() {
        let ex = Exclusions::new(&[r"D:\site\_old".into()]);
        let root = Path::new(r"D:\site");
        assert!(is_reachable(Path::new(r"D:\site\docs\a.txt"), root, &ex));
        assert!(!is_reachable(Path::new(r"D:\site\node_modules\x\a.js"), root, &ex));
        assert!(!is_reachable(Path::new(r"D:\site\.git\HEAD"), root, &ex));
        assert!(!is_reachable(Path::new(r"D:\site\_old\a.txt"), root, &ex));
        assert!(!is_reachable(Path::new(r"E:\other\a.txt"), root, &ex));
    }
}
