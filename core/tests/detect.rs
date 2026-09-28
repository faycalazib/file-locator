//! Lot 6.2: detectors and the personal-data audit, through the index and the
//! live scan, on a folder made for the test in target/tmp.

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use prospector_core::detect::Detector;
use prospector_core::{live_scan, Engine, Hit, ScanTarget, SearchRequest};

fn names(hits: &[Hit]) -> Vec<String> {
    let mut out: Vec<String> = hits.iter().map(|h| Path::new(&h.path).file_name().unwrap().to_string_lossy().into_owned()).collect();
    out.sort();
    out
}

#[test]
fn detectors_find_validated_data_only() {
    let files = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let root = files.path();
    std::fs::write(root.join("rh-dupont.txt"), "Salarié Dupont. IBAN FR76 3000 6000 0112 3456 7890 189. Mail : dupont@exemple.fr, tél. 06 12 34 56 78.").unwrap();
    std::fs::write(root.join("paie.txt"), "Carte 4111 1111 1111 1111. Sécurité sociale : 1 85 05 78 006 084 91.").unwrap();
    std::fs::write(root.join("neutre.txt"), "Commande 1234 5678 9012 3456 ; référence FR76 3000 6000 0112 3456 7890 188.").unwrap();

    let engine = Engine::open(data.path()).unwrap();
    let site = engine.add_site("Audit", vec![root.to_string_lossy().into_owned()]).unwrap();
    engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    let ids = std::slice::from_ref(&site.id);

    let scanned = |req: &SearchRequest| {
        let hits = Mutex::new(Vec::new());
        let target = ScanTarget { site_id: "live".into(), roots: vec![root.to_path_buf()] };
        live_scan(&[target], &[], req, &AtomicBool::new(false), &|h| hits.lock().unwrap().push(h), &|_, _| {}).unwrap();
        hits.into_inner().unwrap()
    };
    let both = |req: SearchRequest| {
        let indexed = engine.search(ids, &req).unwrap().hits;
        assert_eq!(names(&indexed), names(&scanned(&req)), "index and live scan disagree");
        names(&indexed)
    };
    let with = |detectors: &[Detector], query: &str| SearchRequest { detectors: detectors.to_vec(), query: query.into(), ..Default::default() };

    // An invalid IBAN key and a number with no card brand are not reported.
    assert_eq!(both(with(&[Detector::Iban], "")), ["rh-dupont.txt"]);
    assert_eq!(both(with(&[Detector::Card], "")), ["paie.txt"]);
    // Several detectors: any of them; with words: and.
    assert_eq!(both(with(&[Detector::Iban, Detector::Nir], "")), ["paie.txt", "rh-dupont.txt"]);
    assert_eq!(both(with(&Detector::ALL, "Dupont")), ["rh-dupont.txt"]);

    // The audit: counts per kind, on each file and in total.
    let audit = engine.search(ids, &with(&Detector::ALL, "")).unwrap();
    let rh = audit.hits.iter().find(|h| h.path.ends_with("rh-dupont.txt")).unwrap();
    assert_eq!(rh.detections.get("iban"), Some(&1));
    assert_eq!(rh.detections.get("email"), Some(&1));
    assert_eq!(rh.detections.get("phone"), Some(&1));
    assert!(rh.snippets.iter().any(|s| s.text.contains("⟦FR76 3000 6000 0112 3456 7890 189⟧")), "highlighted");
    // Totals cover every file, even beyond the limit shown.
    let one = engine.search(ids, &SearchRequest { limit: Some(1), ..with(&Detector::ALL, "") }).unwrap();
    assert_eq!(one.hits.len(), 1);
    assert_eq!(one.total_files, 2);
    assert_eq!(one.detections.get("card").map(|t| t.files), Some(1));
    assert_eq!(one.detections.get("iban").map(|t| (t.files, t.matches)), Some((1, 1)));
    assert!(!one.detections.contains_key("vat"));
}
