//! Lot 7.1: the real prospector-cli program on test_fixtures, with its own
//! data folder (`--data`): outputs, exit codes, scan, index, sites.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::AtomicBool;
use std::sync::OnceLock;

use prospector_core::Engine;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("test_fixtures").canonicalize().unwrap()
}

/// A data folder with one indexed site, "Dossiers", on fr/ and archives/.
fn data() -> &'static Path {
    static DATA: OnceLock<tempfile::TempDir> = OnceLock::new();
    DATA.get_or_init(|| {
        let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let engine = Engine::open(dir.path()).unwrap();
        let roots = ["fr", "archives"].map(|d| fixtures().join(d).to_string_lossy().into_owned());
        let site = engine.add_site("Dossiers", roots.to_vec()).unwrap();
        engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
        engine.create_site_group("Travail", vec![site.id.clone()]).unwrap();
        dir
    })
    .path()
}

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_prospector-cli")).arg("--data").arg(data()).args(args).output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn search_in_text_json_and_csv() {
    let text = cli(&["search", "contrat", "--site", "dossiers"]);
    assert_eq!(text.status.code(), Some(0), "{}", String::from_utf8_lossy(&text.stderr));
    let out = stdout(&text);
    assert!(out.contains("notes-contrat.txt") && out.contains("[contrat]"), "{out}");

    let json: serde_json::Value = serde_json::from_str(&stdout(&cli(&["search", "contrat", "--format", "json"]))).unwrap();
    let hits = json["hits"].as_array().unwrap();
    assert!(!hits.is_empty() && json["totalFiles"].as_u64().unwrap() >= hits.len() as u64);
    let first = &hits[0];
    assert_eq!(first["site"], "Dossiers");
    assert!(first["modified"].as_str().unwrap().ends_with('Z'));
    assert!(!first["snippets"][0]["text"].as_str().unwrap().contains('⟦'), "no marks in JSON");

    let csv = stdout(&cli(&["search", "contrat", "--kind", "word", "--format", "csv"]));
    let mut lines = csv.lines();
    assert!(lines.next().unwrap().starts_with("path,site,kind"));
    assert!(lines.clone().count() >= 1 && lines.all(|l| l.contains(",word,")), "{csv}");
}

#[test]
fn names_filters_and_archives() {
    let xlsx = stdout(&cli(&["search", "", "--name", "*.xlsx", "--format", "jsonl"]));
    let paths: Vec<String> = xlsx.lines().map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()["path"].as_str().unwrap().to_owned()).collect();
    assert_eq!(paths.len(), 1);
    assert!(paths[0].ends_with("factures-2024.xlsx"));
    // A document inside an archive, with its inner kind.
    let rar = stdout(&cli(&["search", "terrassement", "--format", "jsonl"]));
    let hit: serde_json::Value = serde_json::from_str(rar.lines().next().unwrap()).unwrap();
    assert!(hit["path"].as_str().unwrap().contains(" › devis/terrassement.txt"));
    assert_eq!(hit["innerKind"], "text");
}

#[test]
fn a_terms_file() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let list = dir.path().join("termes.csv");
    std::fs::write(&list, "terme;note\nterrassement;x\n# commentaire\npalissade;y\n").unwrap();
    let out = cli(&["search", "", "--terms-file", list.to_str().unwrap(), "--format", "jsonl"]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let paths: Vec<String> = stdout(&out).lines().map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()["path"].as_str().unwrap().to_owned()).collect();
    assert_eq!(paths.len(), 2, "{paths:?}");
    // All of them: no file has both.
    assert_eq!(cli(&["search", "", "--terms-file", list.to_str().unwrap(), "--terms-all"]).status.code(), Some(1));
    let empty = dir.path().join("vide.txt");
    std::fs::write(&empty, "# rien\n").unwrap();
    assert_eq!(cli(&["search", "", "--terms-file", empty.to_str().unwrap()]).status.code(), Some(2));
}

/// Lot 8.3: meaning search needs the module (this data folder has none).
#[test]
fn meaning_without_the_module() {
    let out = cli(&["search", "bail", "--meaning"]);
    assert_eq!(out.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&out.stderr).contains("meaning module is not installed"));
}

#[test]
fn exit_codes() {
    assert_eq!(cli(&["search", "xyzzyquux"]).status.code(), Some(1), "nothing found");
    assert_eq!(cli(&["search", "contrat", "--kind", "papyrus"]).status.code(), Some(2), "unknown kind");
    assert_eq!(cli(&["search", "(", "--regex"]).status.code(), Some(2), "invalid regex");
    assert_eq!(cli(&["search", "contrat", "--since", "hier"]).status.code(), Some(2), "not a date");
    assert_eq!(cli(&["frobnicate"]).status.code(), Some(2), "unknown command");
    let unknown = cli(&["search", "contrat", "--site", "Nulle part"]);
    assert_eq!(unknown.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unknown site"));
    assert_eq!(cli(&["scan", "x", r"Z:\nowhere\at\all"]).status.code(), Some(3), "missing folder");
}

#[test]
fn scan_without_an_index() {
    let archives = fixtures().join("archives");
    let out = cli(&["scan", "palissade", archives.to_str().unwrap(), "--format", "jsonl"]);
    assert_eq!(out.status.code(), Some(0));
    let lines = stdout(&out);
    let hit: serde_json::Value = serde_json::from_str(lines.lines().next().unwrap()).unwrap();
    assert!(hit["path"].as_str().unwrap().ends_with("courrier-chantier.7z › lettres/relance-fournisseur.txt"));
    let csv = stdout(&cli(&["scan", "xyzzyquux", archives.to_str().unwrap(), "--format", "csv"]));
    assert!(csv.starts_with("path,site,kind") && csv.lines().count() == 1, "an empty CSV keeps its header");
}

#[test]
fn sites_and_index() {
    let sites: serde_json::Value = serde_json::from_str(&stdout(&cli(&["sites", "--format", "json"]))).unwrap();
    assert_eq!(sites[0]["name"], "Dossiers");
    let before = sites[0]["documents"].as_u64().unwrap();
    assert!(before > 0);
    let index = cli(&["index", "Dossiers"]);
    assert_eq!(index.status.code(), Some(0), "{}", String::from_utf8_lossy(&index.stderr));
    assert!(String::from_utf8_lossy(&index.stderr).contains(&format!("Dossiers: {before} documents")));
    assert_eq!(cli(&["index"]).status.code(), Some(2), "a site or --all");
    // Groups (lot 7.2): listed with the sites, usable instead of sites.
    assert_eq!(sites[0]["groups"][0], "Travail");
    assert_eq!(cli(&["search", "contrat", "--group", "travail"]).status.code(), Some(0));
    assert_eq!(cli(&["search", "contrat", "--group", "Nulle part"]).status.code(), Some(3));
    assert_eq!(cli(&["index", "--group", "Travail"]).status.code(), Some(0));
}

/// Lot 7.3: another PC holds the lease of a shared data folder.
#[test]
fn a_shared_index_held_elsewhere_is_read_only() {
    use prospector_core::share::{now, Lease, Role};
    let shared = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let engine = Engine::open(shared.path()).unwrap();
    let site = engine.add_site("Chantier", vec![fixtures().join("fr").to_string_lossy().into_owned()]).unwrap();
    engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    drop(engine);
    assert_eq!(Lease::with_pc(shared.path(), "BUREAU", "other").claim(now()), Role::Maintainer);

    let run = |args: &[&str]| Command::new(env!("CARGO_BIN_EXE_prospector-cli")).arg("--data").arg(shared.path()).args(args).output().unwrap();
    assert_eq!(run(&["search", "contrat"]).status.code(), Some(0), "reading is fine");
    let index = run(&["index", "--all"]);
    assert_eq!(index.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&index.stderr).contains("BUREAU keeps this shared index up to date"));
}
