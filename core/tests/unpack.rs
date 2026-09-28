//! Lot 6.7: extracting a document from an archive or an e-mail, on
//! test_fixtures: the bytes are the original ones, every document found
//! inside a container can be extracted, and the group copy extracts them.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use prospector_core::copy::copy_to_folder;
use prospector_core::extract::{extract_raw, INNER_SEP};
use prospector_core::unpack::{clear_opened, extract_to, is_extractable, open_copy, read};
use prospector_core::{Engine, SearchRequest};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("test_fixtures").canonicalize().unwrap()
}

/// `archives/a.zip`, `x/y.txt` → the full result path.
fn inner(file: &str, inner: &str) -> String {
    format!("{}{INNER_SEP}{inner}", fixtures().join(file).to_string_lossy())
}

/// An entry of a ZIP on disk, read with the zip crate (the reference).
fn zip_entry(file: &Path, name: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(file).unwrap()).unwrap();
    let mut bytes = Vec::new();
    archive.by_name(name).unwrap().read_to_end(&mut bytes).unwrap();
    bytes
}

#[test]
fn archive_entries_come_out_byte_for_byte() {
    let zip = fixtures().join("archives").join("dossier-clients.zip");
    let (name, bytes) = read(&inner("archives/dossier-clients.zip", "annexes/annexe-tarifaire.docx")).unwrap();
    assert_eq!(name, "annexe-tarifaire.docx");
    assert_eq!(bytes, zip_entry(&zip, "annexes/annexe-tarifaire.docx"));

    // Nested: the inner ZIP is opened in memory.
    let ancien = zip_entry(&zip, "archives/ancien.zip");
    let mut nested = zip::ZipArchive::new(std::io::Cursor::new(ancien)).unwrap();
    let mut expected = Vec::new();
    nested.by_name("vieux-contrat-1998.txt").unwrap().read_to_end(&mut expected).unwrap();
    let (name, bytes) = read(&inner("archives/dossier-clients.zip", &format!("archives/ancien.zip{INNER_SEP}vieux-contrat-1998.txt"))).unwrap();
    assert_eq!((name.as_str(), bytes), ("vieux-contrat-1998.txt", expected));

    // RAR, 7z, tar.gz: the text indexed is in the bytes extracted.
    for (file, entry, word) in [
        ("archives/devis-chantier.rar", "devis/terrassement.txt", "terrassement"),
        ("archives/courrier-chantier.7z", "lettres/relance-fournisseur.txt", "palissade"),
        ("archives/sauvegarde.tar.gz", "rapports/bilan.txt", ""),
    ] {
        let (_, bytes) = read(&inner(file, entry)).unwrap_or_else(|e| panic!("{file} › {entry}: {e}"));
        assert!(!bytes.is_empty(), "{file} › {entry}");
        assert!(String::from_utf8_lossy(&bytes).to_lowercase().contains(word), "{file} › {entry}");
    }
    let (_, bilan) = read(&inner("archives/sauvegarde.tar.gz", "rapports/bilan.txt")).unwrap();
    assert_eq!(bilan.len(), 55);
}

#[test]
fn attachments_come_out_as_files() {
    // .eml: an attached Word file, and a text inside an attached ZIP.
    let (name, docx) = read(&inner("mail/devis-charpente.eml", "devis-charpente.docx")).unwrap();
    assert_eq!(name, "devis-charpente.docx");
    assert_eq!(&docx[..2], b"PK");
    let (_, notice) = read(&inner("mail/devis-charpente.eml", &format!("plans.zip{INNER_SEP}plans/notice.txt"))).unwrap();
    assert!(String::from_utf8_lossy(&notice).contains("voliges"));
    // .msg: an old Word document (OLE file).
    let (_, doc) = read(&inner("mail/avec-piece-jointe.msg", "loan_proposal.doc")).unwrap();
    assert_eq!(&doc[..4], &[0xD0, 0xCF, 0x11, 0xE0]);
    // mbox: the second message of the same subject.
    let (_, bon) = read(&inner("mail/fournisseur.mbox", &format!("Commande de tuiles (2){INNER_SEP}bon.txt"))).unwrap();
    assert!(String::from_utf8_lossy(&bon).contains("faîtage"));
}

#[test]
fn what_cannot_be_extracted_says_so() {
    // A message is not a file.
    let message = inner("mail/fournisseur.mbox", "Commande de tuiles");
    assert!(!is_extractable(&message));
    assert!(read(&message).is_err());
    // A name that is not in the archive (renamed or removed since indexing).
    assert!(read(&inner("archives/dossier-clients.zip", "notes/absent.txt")).is_err());
    // A file on disk is not "inside" anything.
    assert!(read(&fixtures().join("fr").join("notes-contrat.txt").to_string_lossy()).is_err());
    // Several documents of one archive in one reading.
    let zip = fixtures().join("archives").join("dossier-clients.zip");
    let found = extract_raw(&zip, &["notes/devis.txt", "annexes/annexe-tarifaire.docx", "nope.txt"]).unwrap();
    assert_eq!(found.len(), 2);
}

/// Every document the index finds inside an archive or an e-mail of the
/// fixtures can be extracted: the inner paths of the capture are the indexed ones.
#[test]
fn every_indexed_document_inside_a_container_can_be_extracted() {
    let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let engine = Engine::open(data.path()).unwrap();
    let roots = ["archives", "mail"].map(|d| fixtures().join(d).to_string_lossy().into_owned());
    let site = engine.add_site("Conteneurs", roots.to_vec()).unwrap();
    let site = engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    let request = SearchRequest { name_pattern: "*".into(), limit: Some(1000), ..Default::default() };
    let hits = engine.search(&[site.id], &request).unwrap().hits;
    let inside: Vec<&str> = hits.iter().map(|h| h.path.as_str()).filter(|p| is_extractable(p)).collect();
    assert!(inside.len() >= 10, "{inside:?}");
    for path in inside {
        assert!(read(path).is_ok(), "cannot extract {path}");
    }
}

#[test]
fn extract_open_and_copy() {
    let out = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let devis = inner("archives/dossier-clients.zip", "notes/devis.txt");
    let expected = zip_entry(&fixtures().join("archives").join("dossier-clients.zip"), "notes/devis.txt");

    // "Extract to…"
    let target = out.path().join("mon-devis.txt");
    extract_to(&devis, &target).unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), expected);

    // "Open": a folder of its own, under the document's name; cleared next time.
    let opened = out.path().join("ouverts");
    let file = open_copy(&devis, &opened).unwrap();
    assert_eq!(file.file_name().unwrap(), "devis.txt");
    assert_eq!(std::fs::read(&file).unwrap(), expected);
    clear_opened(&opened);
    assert_eq!(std::fs::read_dir(&opened).unwrap().count(), 0);

    // Group copy: documents extracted (flat), or the container (unticked).
    let paths = vec![devis.clone(), inner("mail/devis-charpente.eml", "devis-charpente.docx")];
    let never = AtomicBool::new(false);
    let flat = out.path().join("extraits");
    let report = copy_to_folder(&paths, &flat, false, true, &never, &|_, _| {}).unwrap();
    assert_eq!((report.copied, report.from_containers, report.skipped.len()), (2, 0, 0));
    assert_eq!(std::fs::read(flat.join("devis.txt")).unwrap(), expected);
    assert!(flat.join("devis-charpente.docx").exists());
    let tree = out.path().join("arbre");
    copy_to_folder(&paths, &tree, true, true, &never, &|_, _| {}).unwrap();
    assert!(tree.join("archives").join("dossier-clients.zip").join("notes").join("devis.txt").is_file());
    let whole = out.path().join("conteneurs");
    let report = copy_to_folder(&paths, &whole, false, false, &never, &|_, _| {}).unwrap();
    assert_eq!((report.copied, report.from_containers), (2, 2));
    assert!(whole.join("dossier-clients.zip").is_file());
}
