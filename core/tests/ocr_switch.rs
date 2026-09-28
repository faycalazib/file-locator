//! Lot 5.2: switching OCR on later reads again the images and PDFs that were
//! indexed without it (their manifest stamp says "read without OCR"), with no
//! full rebuild. Its own test binary: the OCR switch is global to the process.
#![cfg(windows)]

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use prospector_core::extract::ocr;
use prospector_core::{Engine, IndexPhase, SearchRequest};

#[test]
fn turning_ocr_on_reads_the_images_again() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_fixtures/images/facture-scannee.png");
    std::fs::copy(fixtures, dir.path().join("facture.png")).unwrap();
    std::fs::write(dir.path().join("notes.txt"), "rien à voir").unwrap();

    let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let engine = Engine::open(data.path()).unwrap();
    let site = engine.add_site("Switch", vec![dir.path().to_string_lossy().into_owned()]).unwrap();
    let read = Mutex::new(0);
    let index = || {
        engine
            .index_site(&site.id, &[], &AtomicBool::new(false), &|p| {
                if matches!(p.phase, IndexPhase::Reading) {
                    *read.lock().unwrap() = p.total;
                }
            })
            .unwrap()
    };
    let found = || {
        let req = SearchRequest { query: "vitrerie".into(), ..Default::default() };
        engine.search(std::slice::from_ref(&site.id), &req).unwrap().hits.len()
    };

    ocr::set_enabled(false);
    assert_eq!(index().doc_count, 1, "OCR off: only the text file");
    assert_eq!(found(), 0);

    ocr::set_enabled(true);
    let record = index();
    assert_eq!(*read.lock().unwrap(), 1, "only the image is read again, not the text file");
    assert_eq!(record.doc_count, 2);
    assert_eq!(found(), 1);

    // Nothing changed since: nothing is read.
    index();
    assert_eq!(*read.lock().unwrap(), 0);
}
