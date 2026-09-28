//! Lot 5.2: text read by OCR (Windows) in images and scanned PDFs, on
//! test_fixtures/images and test_fixtures/scans. Every word looked for here
//! exists only as pixels.
#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, OnceLock};

use prospector_core::extract::ocr;
use prospector_core::{live_scan, Engine, Hit, ScanTarget, SearchRequest, SiteRecord};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("test_fixtures").canonicalize().unwrap()
}

/// Only the OCR folders are indexed: the test stays fast.
fn roots() -> Vec<PathBuf> {
    vec![fixtures().join("images"), fixtures().join("scans")]
}

fn engine() -> &'static (Engine, SiteRecord, tempfile::TempDir) {
    static ENGINE: OnceLock<(Engine, SiteRecord, tempfile::TempDir)> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let engine = Engine::open(data.path()).unwrap();
        let roots = roots().iter().map(|r| r.to_string_lossy().into_owned()).collect();
        let site = engine.add_site("OCR", roots).unwrap();
        let site = engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
        (engine, site, data)
    })
}

fn names(hits: &[Hit]) -> Vec<String> {
    let mut out: Vec<String> =
        hits.iter().map(|h| Path::new(&h.path).file_name().unwrap().to_string_lossy().into_owned()).collect();
    out.sort();
    out
}

fn indexed(query: &str) -> Vec<String> {
    let (engine, site, _) = engine();
    let req = SearchRequest { query: query.into(), ..Default::default() };
    names(&engine.search(std::slice::from_ref(&site.id), &req).unwrap().hits)
}

fn scanned(query: &str) -> Vec<String> {
    let hits = Mutex::new(Vec::new());
    let targets: Vec<ScanTarget> = roots().into_iter().map(|r| ScanTarget { site_id: "live".into(), roots: vec![r] }).collect();
    let req = SearchRequest { query: query.into(), ..Default::default() };
    live_scan(&targets, &[], &req, &AtomicBool::new(false), &|h| hits.lock().unwrap().push(h), &|_, _| {}).unwrap();
    names(&hits.into_inner().unwrap())
}

fn both(query: &str) -> Vec<String> {
    let from_index = indexed(query);
    assert_eq!(from_index, scanned(query), "index and live scan disagree on {query:?}");
    from_index
}

#[test]
fn ocr_is_available_on_this_machine() {
    assert!(ocr::enabled(), "Windows OCR is needed for these tests; languages: {:?}", ocr::languages());
}

#[test]
fn text_in_png_and_jpeg_images_is_found() {
    assert_eq!(both("vitrerie"), ["facture-scannee.png"]);
    assert_eq!(both("horticulture"), ["recu.jpg"]);
}

#[test]
fn arabic_text_in_an_image_is_found() {
    if !ocr::languages().iter().any(|l| l.starts_with("ar")) {
        eprintln!("Arabic OCR not installed in Windows: skipped");
        return;
    }
    assert_eq!(both("المصعد"), ["اعلان.png"]);
}

#[test]
fn a_scanned_pdf_is_read_page_by_page() {
    assert_eq!(both("toiture"), ["courrier-scanne.pdf"]);
    let (engine, site, _) = engine();
    let hit = &engine.search(std::slice::from_ref(&site.id), &SearchRequest { query: "toiture".into(), ..Default::default() }).unwrap().hits[0];
    assert_eq!(hit.kind, "pdf");
    assert!(!hit.snippets.is_empty(), "the OCR text is shown in the snippets");
}

#[test]
fn icons_are_not_read() {
    let (_, site, _) = engine();
    // facture, recu, اعلان and the scanned PDF; icone.png (64 px) is ignored.
    assert_eq!(site.doc_count, 4, "{:?}", site.skipped);
    let req = SearchRequest { name_pattern: "icone*".into(), ..Default::default() };
    assert!(engine().0.search(std::slice::from_ref(&site.id), &req).unwrap().hits.is_empty());
}

/// Lot 6.6: the words found are located on the image (boxes as fractions).
#[test]
fn words_found_are_boxed_on_the_image() {
    let image = fixtures().join("images").join("facture-scannee.png").to_string_lossy().into_owned();
    let req = |q: &str| SearchRequest { query: q.into(), fuzzy: false, ..Default::default() };
    let boxes = engine().0.image_matches(&image, &req("vitrerie")).unwrap();
    assert!(!boxes.is_empty(), "the word is on the image");
    for b in &boxes {
        assert!(b.x >= 0.0 && b.y >= 0.0 && b.x + b.w <= 1.01 && b.y + b.h <= 1.01 && b.w > 0.0 && b.h > 0.0, "{b:?}");
        assert!(!b.fuzzy);
    }
    assert!(engine().0.image_matches(&image, &req("introuvablemot")).unwrap().is_empty());
    // Arabic image, Arabic word.
    let arabic = fixtures().join("images").join("اعلان.png").to_string_lossy().into_owned();
    assert!(!engine().0.image_matches(&arabic, &req("المصعد")).unwrap().is_empty());
}

