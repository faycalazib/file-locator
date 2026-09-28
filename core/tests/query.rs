//! Étape 5: query features beyond the text (file-name criterion, folders…),
//! on test_fixtures/, through the index and through the live scan.

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

fn req(query: &str, names: &str) -> SearchRequest {
    SearchRequest { query: query.into(), name_pattern: names.into(), fuzzy: false, ..Default::default() }
}

/// Paths relative to test_fixtures/, forward slashes, sorted.
fn rels(hits: &[Hit]) -> Vec<String> {
    let root = fixtures().to_string_lossy().into_owned();
    let mut out: Vec<String> = hits
        .iter()
        .map(|h| h.path.strip_prefix(&root).unwrap_or(&h.path).trim_start_matches(['\\', '/']).replace('\\', "/"))
        .collect();
    out.sort();
    out
}

fn indexed(request: SearchRequest) -> Vec<String> {
    let (engine, site, _) = engine();
    rels(&engine.search(std::slice::from_ref(&site.id), &request).unwrap().hits)
}

fn scanned(request: SearchRequest) -> Vec<String> {
    let hits = Mutex::new(Vec::new());
    let target = ScanTarget { site_id: "live".into(), roots: vec![fixtures()] };
    live_scan(&[target], &[], &request, &AtomicBool::new(false), &|h| hits.lock().unwrap().push(h), &|_, _| {}).unwrap();
    rels(&hits.into_inner().unwrap())
}

/// Same answer from the index and from the live scan.
fn both(request: SearchRequest) -> Vec<String> {
    let from_index = indexed(request.clone());
    assert_eq!(from_index, scanned(request), "index and live scan disagree");
    from_index
}

#[test]
fn a_name_pattern_alone_lists_files_without_text() {
    assert_eq!(both(req("", "*.xlsx")), ["fr/factures-2024.xlsx"]);
    let hits = engine().0.search(&[engine().1.id.clone()], &req("", "*.xlsx")).unwrap().hits;
    assert_eq!((hits[0].match_count, hits[0].snippets.len()), (0, 0), "no text asked, none shown");
}

#[test]
fn name_and_text_combine() {
    // "contrat" in a Word file: the French one, the Spanish one ("contrato",
    // same stem, one letter apart) and the Word file inside the ZIP.
    assert_eq!(
        both(req("contrat", "*.docx")),
        [
            "archives/dossier-clients.zip › annexes/annexe-tarifaire.docx",
            "es/contrato_servicios.docx",
            "fr/contrat-prestation.docx"
        ]
    );
    // Exclusion: every "contrat" except the PDFs and Word files.
    let without = both(req("contrat", "!*.pdf; !*.docx"));
    assert!(without.iter().all(|p| !p.ends_with(".pdf") && !p.ends_with(".docx")), "{without:?}");
    assert!(without.contains(&"fr/notes-contrat.txt".to_owned()));
}

#[test]
fn names_ignore_case_and_accents_arabic_included() {
    assert_eq!(both(req("", "FACTURES*")), ["fr/factures-2024.xlsx"]);
    assert_eq!(both(req("", "عَقْد*.docx")), ["ar/عقد_إيجار.docx"]);
}

#[test]
fn documents_in_an_archive_answer_to_the_archive_name() {
    let rar = both(req("", "*.rar"));
    assert_eq!(rar, ["archives/devis-chantier.rar › devis/terrassement.txt", "archives/devis-chantier.rar › notes/lisez-moi.md"]);
    // …and to their own name.
    assert_eq!(both(req("", "lisez-moi*")), ["archives/devis-chantier.rar › notes/lisez-moi.md"]);
}

#[test]
fn regular_expression_on_names() {
    assert_eq!(both(req("", r"/^contrat-.*\.pdf$/")), ["fr/contrat-prestation.pdf"]);
    let (engine, site, _) = engine();
    assert!(engine.search(std::slice::from_ref(&site.id), &req("", "/[a/")).is_err(), "invalid regex is an error");
}

#[test]
fn folders_are_found_by_name_but_never_excluded_ones() {
    let folders = SearchRequest { name_pattern: "arch*".into(), folders: true, ..Default::default() };
    let found = both(folders.clone());
    assert_eq!(found, ["archives"]);
    let hits = engine().0.search(&[engine().1.id.clone()], &folders).unwrap().hits;
    assert_eq!(hits[0].kind, "folder");
    // node_modules is excluded by default, its sub-folders too.
    let hidden = SearchRequest { name_pattern: "lib".into(), folders: true, ..Default::default() };
    assert!(both(hidden).is_empty());
    // Without a name pattern, the text typed is used as the folder name.
    let typed = SearchRequest { query: "encodings".into(), folders: true, ..Default::default() };
    assert_eq!(both(typed), ["encodings"]);
}

#[test]
fn near_keeps_files_where_the_words_are_close() {
    let pair = ["fr/proximite-loin.txt", "fr/proximite-proche.txt"];
    assert_eq!(both(req("solive entretoise", "")), pair);
    // "solive … entretoise": 30 characters apart in one file, more than 200 in the other.
    assert_eq!(both(req("solive NEAR entretoise", "")), ["fr/proximite-proche.txt"]);
    assert_eq!(both(req("solive NEAR:500 entretoise", "")), pair);
    assert_eq!(both(req("entretoise NOT solive NEAR entretoise", "")), ["fr/proximite-loin.txt"]);
    // Only the close pair is highlighted.
    let hits = engine().0.search(&[engine().1.id.clone()], &req("solive NEAR entretoise", "")).unwrap().hits;
    assert_eq!(hits[0].match_count, 2);
}

#[test]
fn lines_limits_the_search_to_part_of_each_file() {
    assert_eq!(both(req("LINES:1-2 solive", "")), ["fr/proximite-loin.txt", "fr/proximite-proche.txt"]);
    assert_eq!(both(req("LINES:5+ entretoise", "")), ["fr/proximite-loin.txt"]);
    assert_eq!(both(req("LINES:1-3 (entretoise AND solive)", "")), ["fr/proximite-proche.txt"]);
    // A regex across lines: `(?s)` lets `.` cross the line ends.
    assert_eq!(both(req("/(?s)solive.*entretoise/", "")), ["fr/proximite-loin.txt", "fr/proximite-proche.txt"]);
    assert_eq!(both(req("/solive.*entretoise/", "")), ["fr/proximite-proche.txt"], "same line only without (?s)");
}

#[test]
fn like_tolerates_a_typo_even_with_the_option_off() {
    assert!(both(req("solyve", "")).is_empty());
    assert_eq!(both(req("LIKE solyve", "")), ["fr/proximite-loin.txt", "fr/proximite-proche.txt"]);
}

#[test]
fn search_within_results_keeps_only_those_documents() {
    let (engine, site, _) = engine();
    let first = engine.search(std::slice::from_ref(&site.id), &req("contrat", "")).unwrap().hits;
    // A few of them, one inside an archive (`dossier.zip › annexe.docx`).
    let chosen: Vec<String> =
        first.iter().filter(|h| h.path.contains(" › ") || h.path.ends_with(".docx")).map(|h| h.path.clone()).collect();
    assert!(chosen.len() >= 2 && chosen.iter().any(|p| p.contains(" › ")), "{chosen:?}");
    let within = |query: &str| SearchRequest { within_paths: chosen.clone(), ..req(query, "") };
    let all = rels(&first.iter().filter(|h| chosen.contains(&h.path)).cloned().collect::<Vec<_>>());
    assert_eq!(both(within("contrat")), all);
    // Refining: a second word, only among those documents.
    let refined = both(within("annexe"));
    assert!(refined.iter().all(|p| all.contains(p)), "{refined:?}");
    assert!(both(within("solive")).is_empty(), "the word is elsewhere, not in these results");
    // Name criterion alone, within the results.
    assert_eq!(both(SearchRequest { within_paths: chosen.clone(), ..req("", "*.docx") }).len(), all.iter().filter(|p| p.ends_with(".docx")).count());
}

/// "Search with Prospector" on a folder (lot 5.7), `in_folder`.
fn in_folder(query: &str, names: &str, folder: &Path) -> SearchRequest {
    SearchRequest { in_folder: Some(folder.to_string_lossy().into_owned()), ..req(query, names) }
}

#[test]
fn a_folder_inside_the_site_limits_both_modes_whatever_the_case() {
    let everywhere = both(req("contrat", ""));
    assert!(everywhere.iter().any(|p| !p.starts_with("fr/")), "{everywhere:?}");
    let in_fr = both(in_folder("contrat", "", &fixtures().join("fr")));
    assert!(!in_fr.is_empty() && in_fr.iter().all(|p| p.starts_with("fr/")), "{in_fr:?}");
    assert_eq!(in_fr, everywhere.iter().filter(|p| p.starts_with("fr/")).cloned().collect::<Vec<_>>());
    // The Explorer may spell it differently: same answer.
    let shouted = fixtures().join("FR").to_string_lossy().to_uppercase();
    assert_eq!(both(in_folder("contrat", "", Path::new(&shouted))), in_fr);
    // A name alone, and folders.
    assert!(both(in_folder("", "*.txt", &fixtures().join("fr"))).iter().all(|p| p.starts_with("fr/") && p.ends_with(".txt")));
    // `fr` is not `fr2`, and a folder with no match gives nothing.
    assert!(both(in_folder("contrat", "", &fixtures().join("images"))).is_empty());
}

#[test]
fn a_folder_holding_the_site_searches_all_of_it() {
    let (engine, site, _) = engine();
    let parent = fixtures().parent().unwrap().to_path_buf();
    let all = indexed(req("contrat", ""));
    assert_eq!(indexed(in_folder("contrat", "", &parent)), all);
    // A folder elsewhere: nothing from this site's index.
    let elsewhere = SearchRequest { in_folder: Some(r"Z:\nowhere".into()), ..req("contrat", "") };
    assert!(engine.search(std::slice::from_ref(&site.id), &elsewhere).unwrap().hits.is_empty());
    // The site that holds a folder, for the Explorer (whatever the case).
    let fr = fixtures().join("fr").to_string_lossy().to_lowercase();
    assert_eq!(engine.site_holding(&fr).map(|s| s.id), Some(site.id.clone()));
    assert!(engine.site_holding(&parent.to_string_lossy()).is_none());
}

#[test]
fn a_folder_outside_every_site_is_scanned_on_its_own() {
    let hits = Mutex::new(Vec::new());
    let other = ScanTarget { site_id: "other".into(), roots: vec![fixtures().join("code")] };
    let request = in_folder("contrat", "", &fixtures().join("fr"));
    live_scan(&[other], &[], &request, &AtomicBool::new(false), &|h| hits.lock().unwrap().push(h), &|_, _| {}).unwrap();
    let hits = hits.into_inner().unwrap();
    assert!(!hits.is_empty() && hits.iter().all(|h| h.site_id == prospector_core::scope::FOLDER_SITE), "{hits:?}");
    assert!(rels(&hits).iter().all(|p| p.starts_with("fr/")));
}
