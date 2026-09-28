//! Lot 7.4: term lists. "At least one" and "all", combined with the query,
//! identical through the index and the live scan, a column per term in the
//! keyword report, and the time of a search with 2,000 terms.

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::Instant;

use prospector_core::report::DocRef;
use prospector_core::terms::read_list;
use prospector_core::{live_scan, Engine, ScanTarget, SearchRequest};

fn names(mut paths: Vec<String>) -> Vec<String> {
    paths.sort();
    paths.into_iter().map(|p| Path::new(&p).file_name().unwrap().to_string_lossy().into_owned()).collect()
}

#[test]
fn a_list_of_terms_through_the_index_and_the_live_scan() {
    let files = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let root = files.path();
    std::fs::write(root.join("a.txt"), "Livraison de Dupont SARL.").unwrap();
    std::fs::write(root.join("b.txt"), "Commande à García Hermanos.").unwrap();
    std::fs::write(root.join("c.txt"), "Rien à signaler.").unwrap();
    std::fs::write(root.join("d.txt"), "Facture FR12345678901 de Dupont SARL.").unwrap();
    // The list, in Windows-1252 like a file saved by an old Notepad.
    let list_path = root.join("fournisseurs.lst");
    let (cp1252, _, _) = encoding_rs::WINDOWS_1252.encode("# fournisseurs\nDupont SARL\nGarcía Hermanos\n/FR\\d{11}/\n");
    std::fs::write(&list_path, cp1252).unwrap();
    let list = read_list(&list_path).unwrap();
    assert_eq!(list.terms, ["Dupont SARL", "García Hermanos", "/FR\\d{11}/"]);

    let engine = Engine::open(data.path()).unwrap();
    let site = engine.add_site("Achats", vec![root.to_string_lossy().into_owned()]).unwrap();
    engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    let ids = vec![site.id.clone()];
    let indexed = |req: &SearchRequest| names(engine.search(&ids, req).unwrap().hits.into_iter().map(|h| h.path).collect());
    let scanned = |req: &SearchRequest| {
        let found = Mutex::new(Vec::new());
        let target = ScanTarget { site_id: site.id.clone(), roots: vec![root.to_path_buf()] };
        live_scan(&[target], &[], req, &AtomicBool::new(false), &|h| found.lock().unwrap().push(h.path), &|_, _| {}).unwrap();
        names(found.into_inner().unwrap())
    };
    let both = |req: SearchRequest| {
        let from_index = indexed(&req);
        assert_eq!(from_index, scanned(&req), "index and live scan disagree");
        from_index
    };

    let any = SearchRequest { term_list: list.terms.clone(), ..Default::default() };
    assert_eq!(both(any.clone()), ["a.txt", "b.txt", "d.txt"]);
    // With words in the search box: both must be there.
    assert_eq!(both(SearchRequest { query: "facture".into(), ..any.clone() }), ["d.txt"]);
    // "All": every term of the list.
    let all = SearchRequest { term_list: vec!["Dupont SARL".into(), "/FR\\d{11}/".into()], term_list_all: true, ..Default::default() };
    assert_eq!(both(all), ["d.txt"]);

    // Keyword report: one column per term of the list.
    let docs: Vec<DocRef> = ["a.txt", "b.txt", "d.txt"]
        .iter()
        .map(|n| DocRef { site_id: site.id.clone(), path: root.join(n).to_string_lossy().into_owned() })
        .collect();
    let report = engine.keyword_report(&docs, &any).unwrap();
    assert_eq!(report.terms, ["\"Dupont SARL\"", "\"García Hermanos\"", "/FR\\d{11}/"]);
    assert_eq!(report.totals.iter().map(|t| t.files).collect::<Vec<_>>(), [2, 1, 1]);

    // 2,000 terms: still a search, not a coffee break.
    let mut many: Vec<String> = (0..2000).map(|i| format!("fournisseur{i}")).collect();
    many.push("García Hermanos".into());
    let started = Instant::now();
    let hits = indexed(&SearchRequest { term_list: many, ..Default::default() });
    let took = started.elapsed();
    assert_eq!(hits, ["b.txt"]);
    eprintln!("2,000 terms: {} ms", took.as_millis());
    assert!(took.as_secs() < 10, "{took:?}");
}
