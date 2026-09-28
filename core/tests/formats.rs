//! Étape 2 end-to-end tests: Excel, PowerPoint, ZIP (nested + bomb), Outlook,
//! regular expressions, whole-word option, live scan, preview from disk.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, OnceLock};

use prospector_core::{live_scan, Engine, Hit, ScanTarget, SearchRequest, SiteRecord};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("test_fixtures").canonicalize().unwrap()
}

fn engine() -> &'static (Engine, SiteRecord, tempfile::TempDir) {
    static ENGINE: OnceLock<(Engine, SiteRecord, tempfile::TempDir)> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let engine = Engine::open(data.path()).unwrap();
        let site = engine.add_site("Fixtures", vec![fixtures().to_string_lossy().into_owned()]).unwrap();
        let site = engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
        (engine, site, data)
    })
}

fn hits(req: SearchRequest) -> Vec<Hit> {
    let (engine, site, _) = engine();
    engine.search(std::slice::from_ref(&site.id), &req).unwrap().hits
}

fn q(query: &str) -> SearchRequest {
    SearchRequest { query: query.into(), ..Default::default() }
}

/// Path relative to the fixtures folder, with forward slashes.
fn rel(hit: &Hit) -> String {
    let root = fixtures().to_string_lossy().into_owned();
    hit.path.trim_start_matches(&root).trim_start_matches(['\\', '/']).replace('\\', "/")
}

fn rels(list: &[Hit]) -> Vec<String> {
    let mut v: Vec<String> = list.iter().map(rel).collect();
    v.sort();
    v
}

#[test]
fn excel_sheets_are_searchable() {
    let found = hits(q("renouvellement"));
    assert_eq!(rels(&found), ["fr/factures-2024.xlsx"]);
    assert_eq!(found[0].kind, "excel");
}

#[test]
fn powerpoint_slides_are_searchable() {
    let found = hits(q("tacita"));
    assert_eq!(rels(&found), ["es/presentacion-oferta.pptx"]);
    assert_eq!(found[0].kind, "powerpoint");
}

#[test]
fn sheets_and_slides_get_a_rich_preview() {
    let (engine, site, _) = engine();
    let sheet = &hits(q("renouvellement"))[0];
    let preview = engine.preview(&site.id, &sheet.path, &q("renouvellement")).unwrap();
    assert_eq!(preview.layout, "sheet");
    let texts: Vec<&str> = preview.lines.iter().map(|l| l.text.as_str()).collect();
    assert!(texts.contains(&"— Février —"));
    assert!(texts.contains(&"Référence\tClient\tMontant"), "one cell per column: {texts:?}");

    let deck = &hits(q("tacita"))[0];
    let preview = engine.preview(&site.id, &deck.path, &q("tacita")).unwrap();
    assert_eq!(preview.layout, "slides");
    assert!(preview.lines.iter().any(|l| l.text == "— 2 —"));
}

#[test]
fn zip_entries_become_documents_even_nested() {
    assert!(rels(&hits(q("devis"))).contains(&"archives/dossier-clients.zip › notes/devis.txt".to_owned()));
    assert_eq!(rels(&hits(q("horaire"))), ["archives/dossier-clients.zip › annexes/annexe-tarifaire.docx"]);
    let nested = hits(q("bail"));
    assert_eq!(rels(&nested), ["archives/dossier-clients.zip › archives/ancien.zip › vieux-contrat-1998.txt"]);
    assert_eq!(nested[0].kind, "archive");
}

#[test]
fn rar_and_7z_entries_become_documents() {
    let rar = hits(q("terrassement"));
    assert_eq!(rels(&rar), ["archives/devis-chantier.rar › devis/terrassement.txt"]);
    assert_eq!(rar[0].kind, "archive");
    assert_eq!(rels(&hits(q("bétonnière"))), ["archives/devis-chantier.rar › notes/lisez-moi.md"]);
    // Solid 7z: the entry after the skipped binary one still decodes.
    assert_eq!(rels(&hits(q("palissade"))), ["archives/courrier-chantier.7z › lettres/relance-fournisseur.txt"]);
    let after_binary = hits(q("andamios"));
    assert_eq!(rels(&after_binary), ["archives/courrier-chantier.7z › inventario/almacen.md"]);
    // Entry date from the archive (2024-03-15 10:30 UTC), not the file's.
    assert_eq!(after_binary[0].modified, 1_710_498_600);
}

#[test]
fn password_protected_archives_are_counted_not_indexed() {
    let (_, site, _) = engine();
    assert_eq!(site.skipped.get("encrypted"), Some(&1), "{:?}", site.skipped);
}

#[test]
fn outlook_messages_are_searchable_and_empty_pst_is_harmless() {
    let found = hits(q("\"Test Email\""));
    assert!(rels(&found).contains(&"mail/test_email.msg".to_owned()), "{:?}", rels(&found));
    assert_eq!(found[0].kind, "email");
}

#[test]
fn counts_include_inner_documents_and_skip_the_bomb() {
    let (_, site, _) = engine();
    // 13 (Étape 1) + xlsx + pptx + 3 zip entries + 2 msg; Empty.pst has no message.
    // Étape 3: 2 rar entries + 2 7z entries (the binary one is skipped).
    // Lot 5.2: 3 images and a scanned PDF read by OCR (Windows only).
    let ocr = if prospector_core::extract::ocr::enabled() { 4 } else { 0 };
    // Lots 5.3 and 5.4: 28 more documents (see tests/engine.rs).
    assert_eq!(site.doc_count, 54 + ocr, "skipped: {:?}", site.skipped);
    assert!(hits(q("zeros")).is_empty());
}

#[test]
fn regular_expressions() {
    let whole = hits(SearchRequest { query: r"INV-\d{4}-1[0-9]{2}".into(), regex: true, ..Default::default() });
    assert_eq!(rels(&whole), ["fr/factures-2024.xlsx"]);
    assert!(whole[0].snippets[0].text.contains("⟦INV-2024-117⟧"), "{:?}", whole[0].snippets);

    let inline = hits(q(r"/INV-\d{4}-13\d/ maintenance"));
    assert_eq!(rels(&inline), ["fr/factures-2024.xlsx"]);

    let (engine, site, _) = engine();
    let bad = engine.search(std::slice::from_ref(&site.id), &SearchRequest { query: "(unclosed".into(), regex: true, ..Default::default() });
    let json = serde_json::to_value(bad.unwrap_err()).unwrap();
    assert_eq!(json["code"], "invalidQuery");
}

#[test]
fn whole_word_option() {
    assert!(hits(q("contra")).is_empty());
    let partial = hits(SearchRequest { query: "contra".into(), whole_word: false, ..Default::default() });
    assert!(rels(&partial).contains(&"fr/notes-contrat.txt".to_owned()), "{:?}", rels(&partial));
}

#[test]
fn preview_of_an_inner_document_and_from_disk() {
    let (engine, site, _) = engine();
    let path = format!("{}{}", fixtures().join("archives").join("dossier-clients.zip").to_string_lossy(), " › notes/devis.txt");
    let indexed = engine.preview(&site.id, &path, &q("devis")).unwrap();
    assert!(indexed.lines[0].text.contains("⟦Devis⟧"));
    // Unknown site: read from disk, no index folder created.
    let from_disk = engine.preview("not-a-site", &path, &q("devis")).unwrap();
    assert_eq!(from_disk.lines[0].text, indexed.lines[0].text);
}

#[test]
fn live_scan_streams_hits_without_an_index() {
    let found: Mutex<Vec<Hit>> = Mutex::new(Vec::new());
    let targets = [ScanTarget { site_id: "live".into(), roots: vec![fixtures()] }];
    let summary = live_scan(&targets, &[], &q("contrat NOT résiliation"), &AtomicBool::new(false), &|h| found.lock().unwrap().push(h), &|_, _| {}).unwrap();
    let found = found.into_inner().unwrap();
    assert_eq!(summary.hits, found.len());
    let names = rels(&found);
    assert!(names.contains(&"archives/dossier-clients.zip › notes/devis.txt".to_owned()), "{names:?}");
    assert!(names.contains(&"mail/signature.eml".to_owned()), "{names:?}");
    assert!(!names.contains(&"fr/notes-contrat.txt".to_owned()), "NOT ignored: {names:?}");
    assert!(!names.iter().any(|n| n.contains("node_modules")));
}

/// Diagnostic: time spent extracting each fixture (run with --ignored --nocapture).
#[test]
#[ignore = "diagnostic"]
fn extraction_timings() {
    use prospector_core::extract::extract_docs;
    use prospector_core::kind::FileKind;
    let mut files: Vec<PathBuf> = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() { walk(&p, out) } else { out.push(p) }
        }
    }
    walk(&fixtures(), &mut files);
    for f in files {
        let Some(kind) = FileKind::from_path(&f) else { continue };
        let t = std::time::Instant::now();
        let r = extract_docs(&f, kind);
        println!("{:>8.1?}  {:?}  {}", t.elapsed(), r.as_ref().map(Vec::len).map_err(|e| *e), f.display());
    }
}

/// Diagnostic: time of each indexing phase on the fixtures.
#[test]
#[ignore = "diagnostic"]
fn indexing_phase_timings() {
    // PROSPECTOR_DIAG_DIR lets the diagnostic run on another disk.
    let base = std::env::var("PROSPECTOR_DIAG_DIR").unwrap_or_else(|_| env!("CARGO_TARGET_TMPDIR").to_owned());
    let data = tempfile::tempdir_in(base).unwrap();
    let t = std::time::Instant::now();
    let engine = Engine::open(data.path()).unwrap();
    // PROSPECTOR_DIAG_ROOT: index another folder (e.g. part of the perf corpus).
    let root = std::env::var("PROSPECTOR_DIAG_ROOT").unwrap_or_else(|_| fixtures().to_string_lossy().into_owned());
    let site = engine.add_site("T", vec![root]).unwrap();
    println!("open+add: {:.1?}", t.elapsed());
    let last = Mutex::new(std::time::Instant::now());
    engine
        .index_site(&site.id, &[], &AtomicBool::new(false), &|p| {
            let mut l = last.lock().unwrap();
            println!("{:>8.1?} since previous → {:?} {}/{}", l.elapsed(), p.phase, p.done, p.total);
            *l = std::time::Instant::now();
        })
        .unwrap();
    println!("after last progress: {:.1?}  total: {:.1?}", last.lock().unwrap().elapsed(), t.elapsed());
}

/// Diagnostic: cost of each pipeline stage per file, on PROSPECTOR_DIAG_ROOT.
#[test]
#[ignore = "diagnostic"]
fn pipeline_stage_timings() {
    use prospector_core::extract::extract_docs;
    use prospector_core::kind::FileKind;
    use prospector_core::lang::detect;
    use std::time::{Duration, Instant};
    let root = PathBuf::from(std::env::var("PROSPECTOR_DIAG_ROOT").expect("PROSPECTOR_DIAG_ROOT"));
    let files: Vec<PathBuf> = std::fs::read_dir(&root).unwrap().flatten().map(|e| e.path()).collect();
    let (mut t_extract, mut t_detect) = (Duration::ZERO, Duration::ZERO);
    let mut texts = Vec::new();
    for f in &files {
        let t = Instant::now();
        let docs = extract_docs(f, FileKind::from_path(f).unwrap()).unwrap();
        t_extract += t.elapsed();
        let t = Instant::now();
        let _ = detect(&docs[0].text);
        t_detect += t.elapsed();
        texts.push(docs[0].text.clone());
    }
    let t = Instant::now();
    let mut analyzer = prospector_core::lang::generic_analyzer();
    for text in &texts {
        let _ = prospector_core::lang::analyze(&mut analyzer, text);
    }
    let t_tokenize = t.elapsed();
    let n = files.len() as u32;
    println!("{n} files — extract {:.2?}/file · detect {:.2?}/file · tokenize {:.2?}/file", t_extract / n, t_detect / n, t_tokenize / n);
}
