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
//!
//! Each run has its own worker threads (BUG-068): in the shared rayon pool, a
//! site that finished could wait for a worker busy with a long file of
//! another site (a 2 GB ZIP of scanned books) and stay at "100 %".

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::sync_channel;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::crawler::FileEntry;
use crate::extract::{extract_docs, extract_docs_from_bytes, is_supported, preloadable, ExtractedDoc, SkipReason};

/// Files read ahead of the workers (bounded: memory stays small).
const READ_AHEAD: usize = 256;

/// A file read for longer than this is named in the progress (BUG-068).
pub const SLOW_FILE: Duration = Duration::from_secs(3);

/// Calls `work` for every file with its extracted documents, in parallel.
/// `files` is sorted by path first (sequential reads on disk).
pub fn process_files<F>(files: &mut [FileEntry], cancel: &AtomicBool, work: F)
where
    F: Fn(&FileEntry, Result<Vec<ExtractedDoc>, SkipReason>) + Sync,
{
    process_files_watched(files, cancel, SLOW_FILE, |_| {}, work);
}

/// [`process_files`], and `slow` is told the file read for the longest time
/// once it passes `after` (`None` when no file is that slow any more).
pub fn process_files_watched<F, S>(files: &mut [FileEntry], cancel: &AtomicBool, after: Duration, slow: S, work: F)
where
    F: Fn(&FileEntry, Result<Vec<ExtractedDoc>, SkipReason>) + Sync,
    S: Fn(Option<&Path>) + Sync,
{
    files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    process_stream(files.iter().cloned(), cancel, after, slow, work);
}

/// Calls `work` for every file of `entries`, read in that order by one
/// thread, extracted in parallel. The live scan hands the files over as the
/// walk finds them (BUG-073).
pub fn process_stream<I, F, S>(entries: I, cancel: &AtomicBool, after: Duration, slow: S, work: F)
where
    I: IntoIterator<Item = FileEntry>,
    I::IntoIter: Send,
    F: Fn(&FileEntry, Result<Vec<ExtractedDoc>, SkipReason>) + Sync,
    S: Fn(Option<&Path>) + Sync,
{
    let entries = entries.into_iter();
    let (tx, rx) = sync_channel::<(usize, FileEntry, Option<std::io::Result<Vec<u8>>>)>(READ_AHEAD);
    // Files being extracted: number, path, start.
    let busy: Mutex<Vec<(usize, PathBuf, Instant)>> = Mutex::new(Vec::new());
    let finished = AtomicBool::new(false);
    let pool = rayon::ThreadPoolBuilder::new()
        .thread_name(|i| format!("prospector-read-{i}"))
        .build()
        .ok();

    std::thread::scope(|scope| {
        scope.spawn(move || {
            for (i, file) in entries.enumerate() {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let bytes = (is_supported(&file.path, file.kind) && preloadable(&file.path, file.kind, file.size))
                    .then(|| std::fs::read(&file.path));
                if tx.send((i, file, bytes)).is_err() {
                    break;
                }
            }
        });

        // Names the slowest file in progress, checked 4 times a second.
        let (busy, finished, slow) = (&busy, &finished, &slow);
        scope.spawn(move || {
            let mut shown: Option<usize> = None;
            while !finished.load(Ordering::Relaxed) && !cancel.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(250));
                let list = lock(busy);
                let oldest = list.iter().min_by_key(|(_, _, start)| *start).filter(|(_, _, start)| start.elapsed() >= after);
                let now = oldest.map(|(i, _, _)| *i);
                if now != shown {
                    slow(oldest.map(|(_, path, _)| path.as_path()));
                    shown = now;
                }
            }
        });

        let run = || {
            rx.into_iter().par_bridge().for_each(|(i, file, bytes)| {
                if cancel.load(Ordering::Relaxed) {
                    return;
                }
                lock(busy).push((i, file.path.clone(), Instant::now()));
                let docs = match bytes {
                    Some(Ok(bytes)) => extract_docs_from_bytes(&file.path, file.kind, &bytes),
                    Some(Err(_)) => Err(SkipReason::Unreadable),
                    None => extract_docs(&file.path, file.kind),
                };
                lock(busy).retain(|(j, _, _)| *j != i);
                work(&file, docs);
            });
        };
        match &pool {
            Some(pool) => pool.install(run),
            None => run(),
        }
        finished.store(true, Ordering::Relaxed);
    });
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}
