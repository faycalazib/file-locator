//! BUG-068: indexing progress and independence between sites; BUG-071: Arabic
//! words with ل attached. Its own test binary: one test blocks the shared rayon pool for a moment.

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Barrier, Mutex};
use std::time::Duration;

use prospector_core::{Engine, IndexPhase};

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap()
}

fn files(root: &Path, n: usize) {
    for i in 0..n {
        std::fs::write(root.join(format!("note-{i:03}.txt")), format!("note numéro {i}")).unwrap();
    }
}

#[test]
fn the_bar_moves_at_least_every_percent() {
    let root = temp_dir();
    files(root.path(), 250);
    let data = temp_dir();
    let engine = Engine::open(data.path()).unwrap();
    let site = engine.add_site("Notes", vec![root.path().to_string_lossy().into_owned()]).unwrap();
    let reads = Mutex::new(Vec::new());
    engine
        .index_site(&site.id, &[], &AtomicBool::new(false), &|p| {
            if p.phase == IndexPhase::Reading && p.done > 0 {
                reads.lock().unwrap().push(p.done);
            }
        })
        .unwrap();
    let reads = reads.into_inner().unwrap();
    // 250 files: a report every 2 files (every 64 before: 3 reports, the
    // bar stuck at 77 % until the end).
    assert!(reads.len() >= 100, "{} reports: {reads:?}", reads.len());
    assert_eq!(reads.iter().max(), Some(&250));
}

#[test]
fn a_busy_shared_pool_does_not_hold_an_indexing() {
    let root = temp_dir();
    files(root.path(), 20);
    let data = temp_dir();
    let engine = Arc::new(Engine::open(data.path()).unwrap());
    let site = engine.add_site("Notes", vec![root.path().to_string_lossy().into_owned()]).unwrap();

    // Every worker of the shared pool busy, like with a 2 GB archive of
    // scanned books in another site.
    let workers = rayon::current_num_threads();
    let started = Arc::new(Barrier::new(workers + 1));
    let (release, wait) = mpsc::channel::<()>();
    let wait = Arc::new(Mutex::new(wait));
    for _ in 0..workers {
        let (started, wait) = (started.clone(), wait.clone());
        rayon::spawn(move || {
            started.wait();
            let _ = wait.lock().unwrap().recv_timeout(Duration::from_secs(60));
        });
    }
    started.wait();

    let (done, finished) = mpsc::channel();
    let id = site.id.clone();
    let indexer = engine.clone();
    std::thread::spawn(move || {
        let record = indexer.index_site(&id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
        let _ = done.send(record.doc_count);
    });
    let result = finished.recv_timeout(Duration::from_secs(30));
    for _ in 0..workers {
        let _ = release.send(());
    }
    assert_eq!(result, Ok(20), "the indexing waited for the shared pool");
}

/// BUG-071: in the index too, an Arabic word finds its form with ل attached.
#[test]
fn an_arabic_word_finds_its_form_with_lam_in_the_index() {
    let root = temp_dir();
    std::fs::write(root.path().join("dalil.txt"), "دليل شامل لتصميم المنتجات والإعلانات الاحترافية بالذكاء الاصطناعي").unwrap();
    std::fs::write(root.path().join("lieba.txt"), "لعبة جديدة للأطفال في المدرسة مع أصدقائهم كل يوم").unwrap();
    let data = temp_dir();
    let engine = Engine::open(data.path()).unwrap();
    let site = engine.add_site("Ar", vec![root.path().to_string_lossy().into_owned()]).unwrap();
    engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    let found = |query: &str| -> Vec<String> {
        let req = prospector_core::SearchRequest { query: query.into(), fuzzy: false, ..Default::default() };
        let mut names: Vec<String> = engine
            .search(std::slice::from_ref(&site.id), &req)
            .unwrap()
            .hits
            .into_iter()
            .map(|h| Path::new(&h.path).file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };
    assert_eq!(found("تصميم"), ["dalil.txt"]);
    assert_eq!(found("تصميم NEAR الاصطناعي"), ["dalil.txt"]);
    assert_eq!(found("لعبة"), ["lieba.txt"]);
}

/// BUG-073: the live scan reads files while the walk goes on, and its
/// progress tells both counts (read, found so far).
#[test]
fn the_live_scan_reads_while_walking_and_counts_both() {
    let root = temp_dir();
    files(root.path(), 300);
    let target = prospector_core::ScanTarget { site_id: "scan".into(), roots: vec![root.path().to_path_buf()] };
    let req = prospector_core::SearchRequest { query: "note".into(), fuzzy: false, ..Default::default() };
    let hits = Mutex::new(0);
    let progress = Mutex::new(Vec::new());
    let summary = prospector_core::live_scan(
        &[target],
        &[],
        &req,
        &AtomicBool::new(false),
        &|_| *hits.lock().unwrap() += 1,
        &|read, found| progress.lock().unwrap().push((read, found)),
    )
    .unwrap();
    assert_eq!(*hits.lock().unwrap(), 300);
    assert_eq!(summary.files_scanned, 300);
    let progress = progress.into_inner().unwrap();
    assert_eq!(progress.last(), Some(&(300, 300)), "the last report: everything found and read");
    assert!(progress.iter().all(|(read, found)| read <= found), "never more read than found: {progress:?}");
}
