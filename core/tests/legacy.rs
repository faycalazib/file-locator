//! Lot 5.3: older Office formats (.doc, .ppt — real files from the Apache POI
//! test corpus), RTF, OpenDocument text and presentations, EPUB, TAR / gzip /
//! bzip2 / JAR archives and "protected" PDFs, on test_fixtures/, through the
//! index and through the live scan.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, OnceLock};

use prospector_core::{live_scan, Engine, Hit, ScanTarget, SearchRequest, SiteRecord};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("test_fixtures").canonicalize().unwrap()
}

fn roots() -> Vec<PathBuf> {
    ["legacy", "books", "logs", "archives", "fr"].iter().map(|d| fixtures().join(d)).collect()
}

fn engine() -> &'static (Engine, SiteRecord, tempfile::TempDir) {
    static ENGINE: OnceLock<(Engine, SiteRecord, tempfile::TempDir)> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let engine = Engine::open(data.path()).unwrap();
        let site = engine.add_site("Legacy", roots().iter().map(|r| r.to_string_lossy().into_owned()).collect()).unwrap();
        let site = engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
        (engine, site, data)
    })
}

fn rels(hits: &[Hit]) -> Vec<String> {
    let root = fixtures().to_string_lossy().into_owned();
    let mut out: Vec<String> = hits
        .iter()
        .map(|h| h.path.strip_prefix(&root).unwrap_or(&h.path).trim_start_matches(['\\', '/']).replace('\\', "/"))
        .collect();
    out.sort();
    out
}

fn req(query: &str) -> SearchRequest {
    SearchRequest { query: query.into(), ..Default::default() }
}

fn indexed(request: &SearchRequest) -> Vec<Hit> {
    let (engine, site, _) = engine();
    engine.search(std::slice::from_ref(&site.id), request).unwrap().hits
}

fn scanned(request: &SearchRequest) -> Vec<Hit> {
    let hits = Mutex::new(Vec::new());
    let targets = [ScanTarget { site_id: "live".into(), roots: roots() }];
    live_scan(&targets, &[], request, &AtomicBool::new(false), &|h| hits.lock().unwrap().push(h), &|_, _| {}).unwrap();
    hits.into_inner().unwrap()
}

/// The same files from the index and from the live scan.
fn both(query: &str) -> Vec<String> {
    let from_index = rels(&indexed(&req(query)));
    assert_eq!(from_index, rels(&scanned(&req(query))), "index and live scan disagree on {query:?}");
    from_index
}

#[test]
fn word_97_documents_in_8_bit_and_unicode() {
    assert_eq!(both("boring"), ["legacy/test2.doc"]);
    // UTF-16 text (euro sign, accents) in the header and footer.
    assert_eq!(both("Molière"), ["legacy/HeaderFooterUnicode.doc"]);
    // Cyrillic, also stored as UTF-16.
    assert_eq!(both("Распоряжение"), ["legacy/rasp.doc"]);
}

#[test]
fn powerpoint_97_slides_and_text_boxes_but_not_notes() {
    assert_eq!(both("subtitle"), ["legacy/basic_test_ppt_file.ppt"]);
    assert_eq!(both("Roman"), ["legacy/with_textbox.ppt"]);
    // Speaker notes are not slide content.
    assert!(both("lacking").is_empty());
    let (engine, site, _) = engine();
    let path = indexed(&req("subtitle"))[0].path.clone();
    let preview = engine.preview(&site.id, &path, &req("subtitle")).unwrap();
    assert_eq!(preview.layout, "slides");
    assert!(preview.lines.iter().any(|l| l.text == "— 2 —"), "one block per slide");
}

#[test]
fn rtf_with_accents_and_arabic() {
    assert_eq!(both("bornage"), ["legacy/proces-verbal.rtf"]);
    assert_eq!(both("géomètre"), ["legacy/proces-verbal.rtf"]);
    assert_eq!(both("الورشة"), ["legacy/proces-verbal.rtf"]);
}

#[test]
fn opendocument_text_and_presentation() {
    assert_eq!(both("ardoises"), ["legacy/compte-rendu.odt"]);
    assert_eq!(both("zinguerie"), ["legacy/avancement.odp"]);
    assert!(both("privées").is_empty(), "presentation notes are left out");
}

#[test]
fn epub_chapters_in_order() {
    assert_eq!(both("sémaphore"), ["books/le-phare.epub"]);
    assert_eq!(both("digue"), ["books/le-phare.epub"]);
}

#[test]
fn tar_gzip_bzip2_and_jar() {
    assert_eq!(both("lampadaires"), ["archives/sauvegarde.tar.gz › rapports/bilan.txt"]);
    assert_eq!(both("pare-feu"), ["logs/serveur.log.gz › serveur.log"]);
    assert_eq!(both("orangeraie"), ["logs/notes.txt.bz2 › notes.txt"]);
    assert_eq!(both("javanais"), ["archives/outils.jar › META-INF/README.txt"]);
    // The date of a tar entry comes from the archive.
    assert_eq!(indexed(&req("lampadaires"))[0].modified, 1_710_498_600);
    // Name criterion: documents inside answer to the archive's name.
    let names = SearchRequest { name_pattern: "*.tar.gz".into(), ..Default::default() };
    assert_eq!(
        rels(&indexed(&names)),
        ["archives/sauvegarde.tar.gz › rapports/bilan.txt", "archives/sauvegarde.tar.gz › rapports/equipe.md"]
    );
}

#[test]
fn pdf_protected_only_against_copying_is_read() {
    assert_eq!(both("quinquennale"), ["fr/protege.pdf"]);
}
