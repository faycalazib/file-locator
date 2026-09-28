//! Prospector core engine: crawler, extractors, multilingual Tantivy index,
//! search and highlighting. No Tauri dependency: testable on its own.

pub mod copy;
pub mod crawler;
pub mod detect;
pub mod disk;
pub mod dupes;
pub mod engine;
pub mod error;
pub mod extract;
pub mod facets;
pub mod fsutil;
pub mod groups;
pub mod highlight;
pub mod index;
pub mod kind;
pub mod lang;
pub mod locate;
pub mod manifest;
pub mod names;
pub mod pipeline;
pub mod portable;
pub mod query;
pub mod report;
pub mod saved;
pub mod scan;
pub mod scope;
pub mod search;
pub mod sense;
pub mod share;
pub mod terms;
pub mod thumb;
pub mod unpack;
pub mod userpath;
pub mod watch;

pub use engine::{relocate_data, AlertNews, Embed, Engine, ImageBox, IndexPhase, IndexProgress, PreviewDoc, SearchResponse, SenseHit, SensePlan, SenseProgress, SiteRecord};
pub use error::{CoreError, Result};
pub use groups::SiteGroup;
pub use saved::{Alert, SavedSearch};
pub use scan::{live_scan, ScanSummary, ScanTarget};
pub use search::{Hit, SearchRequest};
pub use watch::{Changes, FolderWatcher};

/// Version of the core engine, exposed to the UI for the "about" panel.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Unit tests' temporary folder, under target/tmp (never the system TEMP).
#[cfg(test)]
pub(crate) fn test_tmp() -> tempfile::TempDir {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("target").join("tmp");
    std::fs::create_dir_all(&base).unwrap();
    tempfile::tempdir_in(base).unwrap()
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert!(!super::version().is_empty());
    }
}
