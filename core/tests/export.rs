//! Lot 6.4: keyword report and "copy the files found" (folder, ZIP).

use std::io::Read;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use prospector_core::copy::{copy_to_folder, copy_to_zip};
use prospector_core::extract::INNER_SEP;
use prospector_core::report::DocRef;
use prospector_core::{Engine, SearchRequest};

#[test]
fn keyword_report_counts_each_term_per_file() {
    let files = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let root = files.path();
    std::fs::write(root.join("a.txt"), "Le contrat, puis la facture, puis encore le contrat.").unwrap();
    std::fs::write(root.join("b.txt"), "Une facture seule, et une facture de plus.").unwrap();
    let engine = Engine::open(data.path()).unwrap();
    let site = engine.add_site("Mots", vec![root.to_string_lossy().into_owned()]).unwrap();
    engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();

    let req = SearchRequest { query: "contrat OR facture".into(), ..Default::default() };
    let hits = engine.search(std::slice::from_ref(&site.id), &req).unwrap().hits;
    let docs: Vec<DocRef> = hits.iter().map(|h| DocRef { site_id: h.site_id.clone(), path: h.path.clone() }).collect();
    let report = engine.keyword_report(&docs, &req).unwrap();
    assert_eq!(report.terms, ["contrat", "facture"]);
    let row = |name: &str| report.rows.iter().find(|r| r.path.ends_with(name)).unwrap().counts.clone();
    assert_eq!(row("a.txt"), [2, 1]);
    assert_eq!(row("b.txt"), [0, 2]);
    assert_eq!((report.totals[0].files, report.totals[0].matches), (1, 2));
    assert_eq!((report.totals[1].files, report.totals[1].matches), (2, 3));

    // A live-scan result (unknown site): read from disk.
    let live = [DocRef { site_id: "live".into(), path: root.join("b.txt").to_string_lossy().into_owned() }];
    assert_eq!(engine.keyword_report(&live, &req).unwrap().rows[0].counts, [0, 2]);
}

fn entries(zip: &Path) -> Vec<(String, String)> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip).unwrap()).unwrap();
    let mut out = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let mut text = String::new();
        entry.read_to_string(&mut text).unwrap();
        out.push((entry.name().to_owned(), text));
    }
    out.sort();
    out
}

#[test]
fn copies_to_a_folder_and_to_a_zip() {
    let src = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let out = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let (a, b) = (src.path().join("A"), src.path().join("B"));
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    std::fs::write(a.join("facture.txt"), "A").unwrap();
    std::fs::write(b.join("facture.txt"), "B").unwrap();
    std::fs::write(b.join("dossier.zip"), "pas vraiment un zip").unwrap();
    let s = |p: &Path| p.to_string_lossy().into_owned();
    let paths = vec![
        s(&a.join("facture.txt")),
        s(&b.join("facture.txt")),
        format!("{}{INNER_SEP}x.pdf", s(&b.join("dossier.zip"))),
        format!("{}{INNER_SEP}y.pdf", s(&b.join("dossier.zip"))),
        s(&b.join("disparu.txt")),
    ];
    let never = AtomicBool::new(false);

    // Flat: same names get " (2)"; the container comes once; a missing file is listed.
    let flat = out.path().join("plat");
    let report = copy_to_folder(&paths, &flat, false, false, &never, &|_, _| {}).unwrap();
    assert_eq!(report.copied, 3);
    assert_eq!(report.from_containers, 2);
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(std::fs::read_to_string(flat.join("facture.txt")).unwrap(), "A");
    assert_eq!(std::fs::read_to_string(flat.join("facture (2).txt")).unwrap(), "B");
    assert!(flat.join("dossier.zip").exists());
    // Again into the same folder: nothing is overwritten.
    copy_to_folder(&paths[..1], &flat, false, false, &never, &|_, _| {}).unwrap();
    assert_eq!(std::fs::read_to_string(flat.join("facture.txt")).unwrap(), "A");
    assert_eq!(std::fs::read_to_string(flat.join("facture (3).txt")).unwrap(), "A", "next free name");

    // Keeping the folders below their common parent.
    let tree = out.path().join("arbre");
    copy_to_folder(&paths[..2], &tree, true, false, &never, &|_, _| {}).unwrap();
    assert_eq!(std::fs::read_to_string(tree.join("A").join("facture.txt")).unwrap(), "A");
    assert_eq!(std::fs::read_to_string(tree.join("B").join("facture.txt")).unwrap(), "B");

    // ZIP, with the folders.
    let zip = out.path().join("trouves.zip");
    let report = copy_to_zip(&paths, &zip, true, false, &never, &|_, _| {}).unwrap();
    assert_eq!(report.copied, 3);
    assert_eq!(
        entries(&zip),
        vec![
            ("A/facture.txt".to_owned(), "A".to_owned()),
            ("B/dossier.zip".to_owned(), "pas vraiment un zip".to_owned()),
            ("B/facture.txt".to_owned(), "B".to_owned()),
        ]
    );
}
