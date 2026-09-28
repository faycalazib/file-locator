//! "Copy the files found" (lot 6.4): to a folder or into a ZIP archive,
//! flat or keeping the folders below their common parent. A document inside
//! an archive or an e-mail is extracted (lot 6.7; each container is read
//! once for all its documents), or, when asked (and for a message of a
//! mailbox, which is not a file), brings its whole container, once. Nothing
//! is ever overwritten: a name already taken gets " (2)", " (3)"…

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::error::{CoreError, Result};
use crate::extract::{extract_raw, inner_name, split_inner, INNER_SEP};
use crate::unpack::{is_extractable, safe_name};

/// What a copy did.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyReport {
    pub copied: usize,
    /// Files that could not be read (gone, locked…).
    pub skipped: Vec<String>,
    /// Results inside an archive or a mailbox, brought by their container.
    pub from_containers: usize,
    /// The folder or the ZIP written.
    pub target: String,
    pub cancelled: bool,
}

/// One thing to copy.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    /// A file on disk (a container brought whole included).
    File(PathBuf),
    /// A document extracted from an archive or an e-mail.
    Inner { container: PathBuf, inner: String },
}

impl Item {
    /// The file on disk it comes from.
    fn file(&self) -> &Path {
        match self {
            Item::File(file) | Item::Inner { container: file, .. } => file,
        }
    }

    fn label(&self) -> String {
        match self {
            Item::File(file) => file.to_string_lossy().into_owned(),
            Item::Inner { container, inner } => format!("{}{INNER_SEP}{inner}", container.to_string_lossy()),
        }
    }
}

/// What to copy for `paths`, each once, in order; and how many results were
/// documents brought by their whole container. `extract`: documents inside
/// containers are extracted (a message of a mailbox still brings it).
pub fn items_of(paths: &[String], extract: bool) -> (Vec<Item>, usize) {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    let mut from_containers = 0;
    for path in paths {
        let (file, part) = split_inner(path);
        let item = match part {
            Some(inner) if extract && is_extractable(path) => Item::Inner { container: PathBuf::from(file), inner: inner.to_owned() },
            Some(_) => {
                from_containers += 1;
                Item::File(PathBuf::from(file))
            }
            None => Item::File(PathBuf::from(file)),
        };
        if seen.insert(item.clone()) {
            items.push(item);
        }
    }
    (items, from_containers)
}

/// The documents extracted from containers: each container is read once for
/// all its documents, which are then forgotten as they are written.
struct Extracted {
    /// Inner paths wanted, per container not read yet.
    wanted: HashMap<PathBuf, Vec<String>>,
    read: HashMap<PathBuf, HashMap<String, Vec<u8>>>,
}

impl Extracted {
    fn new(items: &[Item]) -> Self {
        let mut wanted: HashMap<PathBuf, Vec<String>> = HashMap::new();
        for item in items {
            if let Item::Inner { container, inner } = item {
                wanted.entry(container.clone()).or_default().push(inner.clone());
            }
        }
        Self { wanted, read: HashMap::new() }
    }

    /// The bytes of one document (`None`: not found or unreadable).
    fn take(&mut self, container: &Path, inner: &str) -> Option<Vec<u8>> {
        if let Some(wanted) = self.wanted.remove(container) {
            let refs: Vec<&str> = wanted.iter().map(String::as_str).collect();
            self.read.insert(container.to_path_buf(), extract_raw(container, &refs).unwrap_or_default());
        }
        let found = self.read.get_mut(container)?;
        let bytes = found.remove(inner);
        if found.is_empty() {
            self.read.remove(container);
        }
        bytes
    }
}

/// The deepest folder holding every file (`None`: different drives).
fn common_parent(files: &[&Path]) -> Option<PathBuf> {
    let mut common: Option<PathBuf> = None;
    for file in files {
        let parent = file.parent()?.to_path_buf();
        common = Some(match common {
            None => parent,
            Some(current) => {
                let shared: PathBuf = current.components().zip(parent.components()).take_while(|(a, b)| a == b).map(|(a, _)| a).collect();
                if shared.as_os_str().is_empty() {
                    return None;
                }
                shared
            }
        });
    }
    common
}

/// Where each item goes, relative to the target: its name (flat) or its path
/// below the common parent (the drive letter as a first folder when the
/// files are on several drives). In a tree, a document extracted is placed
/// in a folder named after its container, with its own folders
/// (`clients.zip\notes\devis.txt`). Names taken twice get " (2)"…
fn relative_names(items: &[Item], keep_tree: bool) -> Vec<PathBuf> {
    let files: Vec<&Path> = items.iter().map(Item::file).collect();
    let root = if keep_tree { common_parent(&files) } else { None };
    let mut taken: HashSet<String> = HashSet::new();
    items
        .iter()
        .map(|item| {
            let file = item.file();
            let relative: PathBuf = if let (false, Item::Inner { inner, .. }) = (keep_tree, item) {
                PathBuf::from(safe_name(inner_name(inner)))
            } else if !keep_tree {
                PathBuf::from(file.file_name().unwrap_or_default())
            } else if let Some(rest) = root.as_ref().and_then(|r| file.strip_prefix(r).ok()) {
                rest.to_path_buf()
            } else {
                file.components()
                    .filter_map(|c| match c {
                        Component::Prefix(p) => Some(p.as_os_str().to_string_lossy().trim_end_matches(':').to_owned()),
                        Component::Normal(n) => Some(n.to_string_lossy().into_owned()),
                        _ => None,
                    })
                    .collect()
            };
            let relative = match item {
                Item::Inner { inner, .. } if keep_tree => {
                    let mut path = relative;
                    let steps = inner.split(INNER_SEP).flat_map(|s| s.split(['/', '\\'])).filter(|s| !matches!(*s, "" | "." | ".."));
                    for step in steps {
                        path.push(safe_name(step));
                    }
                    path
                }
                _ => relative,
            };
            unique(relative, &mut taken)
        })
        .collect()
}

/// `name.pdf`, `name (2).pdf`… the first one not taken (case ignored, as on Windows).
fn unique(path: PathBuf, taken: &mut HashSet<String>) -> PathBuf {
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let mut candidate = path.clone();
    let mut n = 2;
    while !taken.insert(candidate.to_string_lossy().to_lowercase()) {
        candidate = path.with_file_name(format!("{stem} ({n}){ext}"));
        n += 1;
    }
    candidate
}

/// Copies to a folder (created if needed); an existing file is never replaced.
pub fn copy_to_folder(
    paths: &[String],
    dest: &Path,
    keep_tree: bool,
    extract: bool,
    cancel: &AtomicBool,
    progress: &dyn Fn(usize, usize),
) -> Result<CopyReport> {
    let (items, from_containers) = items_of(paths, extract);
    let mut extracted = Extracted::new(&items);
    std::fs::create_dir_all(dest)?;
    let mut report = CopyReport { from_containers, target: dest.to_string_lossy().into_owned(), ..Default::default() };
    for (i, (item, relative)) in items.iter().zip(relative_names(&items, keep_tree)).enumerate() {
        if cancel.load(Ordering::Relaxed) {
            report.cancelled = true;
            break;
        }
        // The names are unique among the files copied; not over a file
        // already in the destination either.
        let mut target = dest.join(&relative);
        if target.exists() {
            target = dest.join(unique(relative.clone(), &mut target_names(dest, &relative)));
        }
        let done = target.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| match item {
            Item::File(file) => std::fs::copy(file, &target).map(|_| ()),
            Item::Inner { container, inner } => match extracted.take(container, inner) {
                Some(bytes) => std::fs::write(&target, bytes),
                None => Err(std::io::ErrorKind::NotFound.into()),
            },
        });
        match done {
            Ok(()) => report.copied += 1,
            Err(_) => report.skipped.push(item.label()),
        }
        progress(i + 1, items.len());
    }
    Ok(report)
}

/// The names already present next to `relative` in `dest`.
fn target_names(dest: &Path, relative: &Path) -> HashSet<String> {
    let folder = dest.join(relative).parent().map(Path::to_path_buf).unwrap_or_else(|| dest.to_path_buf());
    std::fs::read_dir(&folder)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| relative.with_file_name(e.file_name()).to_string_lossy().to_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

/// Writes a ZIP archive (replaced if it exists: the user chose it in the
/// save dialog). Unreadable files are left out and listed.
pub fn copy_to_zip(
    paths: &[String],
    zip_path: &Path,
    keep_tree: bool,
    extract: bool,
    cancel: &AtomicBool,
    progress: &dyn Fn(usize, usize),
) -> Result<CopyReport> {
    let (items, from_containers) = items_of(paths, extract);
    let mut extracted = Extracted::new(&items);
    let storage = |e: zip::result::ZipError| CoreError::Storage { message: e.to_string() };
    let mut zip = zip::ZipWriter::new(BufWriter::new(File::create(zip_path)?));
    let mut report = CopyReport { from_containers, target: zip_path.to_string_lossy().into_owned(), ..Default::default() };
    let mut buffer = vec![0u8; 256 * 1024];
    for (i, (item, relative)) in items.iter().zip(relative_names(&items, keep_tree)).enumerate() {
        if cancel.load(Ordering::Relaxed) {
            report.cancelled = true;
            break;
        }
        // A file is streamed from disk; an extracted document is in memory.
        let source = match item {
            Item::File(file) => File::open(file).ok().map(Source::Disk),
            Item::Inner { container, inner } => extracted.take(container, inner).map(Source::Memory),
        };
        let Some(source) = source else {
            report.skipped.push(item.label());
            progress(i + 1, items.len());
            continue;
        };
        let size = match &source {
            Source::Disk(file) => file.metadata().map(|m| m.len()).unwrap_or(0),
            Source::Memory(bytes) => bytes.len() as u64,
        };
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated).large_file(size >= u32::MAX as u64);
        let name = relative.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
        zip.start_file(name, options).map_err(storage)?;
        match source {
            Source::Memory(bytes) => zip.write_all(&bytes)?,
            Source::Disk(mut file) => loop {
                let n = file.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                zip.write_all(&buffer[..n])?;
            },
        }
        report.copied += 1;
        progress(i + 1, items.len());
    }
    zip.finish().map_err(storage)?.flush()?;
    Ok(report)
}

/// Where the bytes of one item come from.
enum Source {
    Disk(File),
    Memory(Vec<u8>),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(paths: &[&str]) -> Vec<Item> {
        paths.iter().map(|p| Item::File(PathBuf::from(p))).collect()
    }

    #[test]
    fn containers_come_once() {
        let paths = vec![r"D:\a\x.pdf".to_owned(), format!(r"D:\a\dossier.zip{INNER_SEP}c.pdf"), format!(r"D:\a\dossier.zip{INNER_SEP}d.pdf")];
        let (items, inner) = items_of(&paths, false);
        assert_eq!(items, files(&[r"D:\a\x.pdf", r"D:\a\dossier.zip"]));
        assert_eq!(inner, 2);
    }

    #[test]
    fn documents_are_extracted_but_messages_bring_their_mailbox() {
        let paths = vec![
            format!(r"D:\a\dossier.zip{INNER_SEP}c.pdf"),
            format!(r"D:\m.pst{INNER_SEP}Inbox{INNER_SEP}Re - devis"),
            format!(r"D:\m.pst{INNER_SEP}Inbox{INNER_SEP}Re - devis{INNER_SEP}devis.docx"),
        ];
        let inner = |c: &str, i: &str| Item::Inner { container: PathBuf::from(c), inner: i.to_owned() };
        let (items, from_containers) = items_of(&paths, true);
        assert_eq!(
            items,
            vec![
                inner(r"D:\a\dossier.zip", "c.pdf"),
                Item::File(PathBuf::from(r"D:\m.pst")),
                inner(r"D:\m.pst", &format!("Inbox{INNER_SEP}Re - devis{INNER_SEP}devis.docx")),
            ]
        );
        assert_eq!(from_containers, 1);
        // Flat: the document's own name; in a tree, the container is a folder.
        let docs = vec![inner(r"D:\C\a.zip", "notes/devis.txt"), inner(r"D:\C\m.eml", "devis.txt"), Item::File(PathBuf::from(r"D:\C\x\y.pdf"))];
        assert_eq!(relative_names(&docs, false), vec![PathBuf::from("devis.txt"), PathBuf::from("devis (2).txt"), PathBuf::from("y.pdf")]);
        assert_eq!(
            relative_names(&docs, true),
            vec![PathBuf::from(r"a.zip\notes\devis.txt"), PathBuf::from(r"m.eml\devis.txt"), PathBuf::from(r"x\y.pdf")]
        );
    }

    #[test]
    fn names_flat_or_below_the_common_parent() {
        let files = files(&[r"D:\Clients\A\facture.pdf", r"D:\Clients\B\facture.pdf", r"D:\Clients\B\Facture.PDF"]);
        let flat = relative_names(&files, false);
        assert_eq!(flat, vec![PathBuf::from("facture.pdf"), PathBuf::from("facture (2).pdf"), PathBuf::from("Facture (3).PDF")]);
        let tree = relative_names(&files, true);
        assert_eq!(tree[0], PathBuf::from(r"A\facture.pdf"));
        assert_eq!(tree[1], PathBuf::from(r"B\facture.pdf"));
        // Two drives: the drive letter is the first folder.
        let drives = relative_names(&[Item::File(PathBuf::from(r"C:\x\a.txt")), Item::File(PathBuf::from(r"D:\y\b.txt"))], true);
        assert_eq!(drives, vec![PathBuf::from(r"C\x\a.txt"), PathBuf::from(r"D\y\b.txt")]);
    }
}
