//! File-processing pipeline shared by indexing and live scan.
//!
//! On a mechanical or USB hard disk, reading thousands of small files from
//! many threads in random order makes the heads seek constantly: indexing
//! 100 000 cold files slowed down to minutes (BUG-025). Here **one thread
//! reads** the files in path order (nearly sequential on disk) and a
//! read-ahead channel feeds the CPU work (extraction, language detection,
//! indexing) to all the other cores.
//! Containers (ZIP, PST) are opened by the workers themselves: they are read
//! entry by entry, not loaded whole.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::sync_channel;

use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::crawler::FileEntry;
use crate::extract::{extract_docs, extract_docs_from_bytes, is_supported, preloadable, ExtractedDoc, SkipReason};

/// Files read ahead of the workers (bounded: memory stays small).
const READ_AHEAD: usize = 256;

/// Calls `work` for every file with its extracted documents, in parallel.
/// `files` is sorted by path first (sequential reads on disk).
pub fn process_files<F>(files: &mut [FileEntry], cancel: &AtomicBool, work: F)
where
    F: Fn(&FileEntry, Result<Vec<ExtractedDoc>, SkipReason>) + Sync,
{
    files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    let files = &*files;
    let (tx, rx) = sync_channel::<(usize, Option<std::io::Result<Vec<u8>>>)>(READ_AHEAD);

    std::thread::scope(|scope| {
        scope.spawn(move || {
            for (i, file) in files.iter().enumerate() {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let bytes = (is_supported(&file.path, file.kind) && preloadable(&file.path, file.kind, file.size))
                    .then(|| std::fs::read(&file.path));
                if tx.send((i, bytes)).is_err() {
                    break;
                }
            }
        });

        rx.into_iter().par_bridge().for_each(|(i, bytes)| {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            let file = &files[i];
            let docs = match bytes {
                Some(Ok(bytes)) => extract_docs_from_bytes(&file.path, file.kind, &bytes),
                Some(Err(_)) => Err(SkipReason::Unreadable),
                None => extract_docs(&file.path, file.kind),
            };
            work(file, docs);
        });
    });
}
