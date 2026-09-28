//! What was indexed, file by file: size and date at indexing time, number of
//! documents produced, or why the file was skipped. Comparing it with the
//! folders tells which files were added, changed or removed since the last
//! update, so only those are read again (incremental indexing, goal.md §4).

use std::collections::BTreeMap;
use std::ops::Bound;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::crawler::FileEntry;
use crate::kind::FileKind;
use crate::error::Result;
use crate::fsutil::write_atomic;

/// Bumped when the schema or the extraction changes: the next update of each
/// site is then a full rebuild (files skipped before may now be readable).
/// 2: Excel rows keep their empty cells (aligned columns in the preview).
/// 3: RAR and 7z archives are read (they were "unsupported" before).
/// 4: `name_raw` field (file-name criterion).
/// 5: images and scanned PDF pages read by OCR.
/// 6: .doc, .ppt, RTF, ODT/ODP, EPUB, TAR/gzip/bzip2/JAR, protected PDFs.
/// 7: e-mail attachments, mbox, ost.
/// 8: Excel dates read as dates (BUG-041); only spreadsheets are read again.
const VERSION: u32 = 8;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileStamp {
    pub size: u64,
    /// Unix seconds.
    pub modified: u64,
    /// Documents indexed from the file (several for a ZIP or a PST).
    pub docs: u32,
    /// Why the file gave no document (`SkipReason` code), if it failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped: Option<String>,
    /// Read while OCR was on. When OCR is switched on later, images and PDFs
    /// read without it are read again.
    #[serde(default)]
    pub ocr: bool,
}

impl FileStamp {
    /// The file on disk is the one that was indexed.
    pub fn matches(&self, file: &FileEntry) -> bool {
        let needs_ocr = !self.ocr && matches!(file.kind, FileKind::Image | FileKind::Pdf) && crate::extract::ocr::enabled();
        self.size == file.size && self.modified == file.modified && !needs_ocr
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    version: u32,
    pub files: BTreeMap<String, FileStamp>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self { version: VERSION, files: BTreeMap::new() }
    }
}

impl Manifest {
    /// `None` when missing, unreadable or written by another version: the
    /// caller then rebuilds the whole index.
    pub fn load(path: &Path) -> Option<Self> {
        let mut manifest: Self = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
        // 7 → 8 changed only spreadsheets: they are read again, not the whole site.
        if manifest.version == 7 {
            manifest.reread(FileKind::Excel);
            manifest.version = VERSION;
        }
        (manifest.version == VERSION).then_some(manifest)
    }

    /// Files of this kind no longer match their stamp: the next update reads
    /// them again (and replaces their documents).
    fn reread(&mut self, kind: FileKind) {
        for (path, stamp) in &mut self.files {
            if FileKind::from_path(Path::new(path)) == Some(kind) {
                stamp.modified = 0;
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        Ok(write_atomic(path, &serde_json::to_vec(self)?)?)
    }

    /// Indexed files equal to `dir` or inside it.
    pub fn under<'a>(&'a self, dir: &'a str) -> impl Iterator<Item = &'a String> + 'a {
        self.files
            .range::<str, _>((Bound::Included(dir), Bound::Unbounded))
            .map(|(path, _)| path)
            .take_while(move |path| path.starts_with(dir))
            .filter(move |path| is_within(path, dir))
    }

    /// Bytes of the files that gave at least one document.
    pub fn indexed_bytes(&self) -> u64 {
        self.files.values().filter(|s| s.docs > 0).map(|s| s.size).sum()
    }

    pub fn doc_count(&self) -> u64 {
        self.files.values().map(|s| u64::from(s.docs)).sum()
    }

    /// Unreadable files, by reason.
    pub fn skipped(&self) -> BTreeMap<String, u64> {
        let mut skipped = BTreeMap::new();
        for reason in self.files.values().filter_map(|s| s.skipped.as_ref()) {
            *skipped.entry(reason.clone()).or_default() += 1;
        }
        skipped
    }
}

const SEPARATORS: [char; 2] = ['\\', '/'];

/// `path` is `dir` itself or inside it (`D:\a` contains `D:\a\b`, not `D:\ab`).
pub fn is_within(path: &str, dir: &str) -> bool {
    path == dir
        || (path.starts_with(dir) && (dir.ends_with(SEPARATORS) || path[dir.len()..].starts_with(SEPARATORS)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(docs: u32, skipped: Option<&str>) -> FileStamp {
        FileStamp { size: 10, modified: 1, docs, skipped: skipped.map(str::to_owned), ocr: false }
    }

    #[test]
    fn folder_contents_and_totals() {
        let mut m = Manifest::default();
        for (path, docs) in [(r"D:\a\x.txt", 1), (r"D:\a\sub\y.zip", 3), (r"D:\ab\z.txt", 1), (r"D:\b.txt", 0)] {
            m.files.insert(path.to_owned(), stamp(docs, (docs == 0).then_some("unreadable")));
        }
        let inside: Vec<_> = m.under(r"D:\a").cloned().collect();
        assert_eq!(inside, [r"D:\a\sub\y.zip", r"D:\a\x.txt"]);
        assert_eq!(m.under(r"D:\a\x.txt").count(), 1);
        assert_eq!(m.doc_count(), 5);
        assert_eq!(m.indexed_bytes(), 30);
        assert_eq!(m.skipped().get("unreadable"), Some(&1));
        assert!(is_within(r"D:\a\b", r"D:\"));
        assert!(!is_within(r"D:\ab", r"D:\a"));
    }

    #[test]
    fn version_7_rereads_only_spreadsheets() {
        let dir = crate::test_tmp();
        let path = dir.path().join("site.manifest.json");
        let mut old = Manifest { version: 7, files: BTreeMap::new() };
        old.files.insert(r"D:\a\ventes.xlsx".into(), stamp(1, None));
        old.files.insert(r"D:\a\notes.txt".into(), stamp(1, None));
        old.save(&path).unwrap();

        let m = Manifest::load(&path).expect("kept, not rebuilt");
        assert_eq!(m.version, VERSION);
        assert_eq!(m.files[r"D:\a\ventes.xlsx"].modified, 0, "read again");
        assert_eq!(m.files[r"D:\a\notes.txt"].modified, 1, "untouched");
        // Older versions: rebuilt.
        Manifest { version: 6, files: BTreeMap::new() }.save(&path).unwrap();
        assert!(Manifest::load(&path).is_none());
    }
}
