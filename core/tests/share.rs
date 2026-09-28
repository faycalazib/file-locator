//! Lot 7.3: one data folder, two "PCs" (two engines). The holder of the
//! lease indexes; the reader sees its work without reopening anything,
//! cannot change the shared index, and keeps its saved searches to itself.

use std::sync::atomic::AtomicBool;

use prospector_core::share::{Lease, Role};
use prospector_core::{CoreError, Engine, SearchRequest};

fn found(engine: &Engine, query: &str) -> usize {
    let ids: Vec<String> = engine.sites().into_iter().map(|s| s.id).collect();
    let request = SearchRequest { query: query.into(), ..Default::default() };
    engine.search(&ids, &request).unwrap().hits.len()
}

#[test]
fn the_reader_follows_what_the_holder_indexes() {
    let shared = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let files = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let (me_a, me_b) = (tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap(), tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap());
    std::fs::write(files.path().join("devis.txt"), "Devis de charpente.").unwrap();

    // Who does what: the lease.
    let mut lease_a = Lease::with_pc(shared.path(), "BUREAU", "a");
    let mut lease_b = Lease::with_pc(shared.path(), "PORTABLE", "b");
    assert_eq!(lease_a.claim(1000), Role::Maintainer);
    let Role::Reader { pc } = lease_b.claim(1001) else { panic!("B reads") };
    assert_eq!(pc, "BUREAU");

    let a = Engine::open_with(shared.path(), me_a.path()).unwrap();
    let b = Engine::open_with(shared.path(), me_b.path()).unwrap();
    b.set_reader_of(Some(pc));

    // A indexes; B finds it at its next search.
    let site = a.add_site("Chantier", vec![files.path().to_string_lossy().into_owned()]).unwrap();
    a.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    assert_eq!(found(&b, "charpente"), 1);

    // A new file indexed by A: B sees it without reopening anything.
    std::fs::write(files.path().join("toiture.txt"), "Charpente et toiture.").unwrap();
    a.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    assert_eq!(found(&b, "charpente"), 2);

    // B cannot change the shared index.
    let refused = |r: Result<(), CoreError>| matches!(r, Err(CoreError::ReadOnly { ref pc }) if pc == "BUREAU");
    assert!(refused(b.add_site("Autre", vec![files.path().to_string_lossy().into_owned()]).map(|_| ())));
    assert!(refused(b.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).map(|_| ())));
    assert!(refused(b.remove_site(&site.id)));

    // Saved searches and groups stay on each PC.
    b.save_search("Charpente", "charpente", serde_json::json!({})).unwrap();
    b.create_site_group("Chantiers", vec![site.id.clone()]).unwrap();
    assert!(a.saved_searches().is_empty() && a.site_groups().is_empty());
    assert!(me_b.path().join("saved-searches.json").exists());
    assert!(!shared.path().join("saved-searches.json").exists());

    // A removes the site: B no longer lists it.
    a.remove_site(&site.id).unwrap();
    assert_eq!(found(&b, "charpente"), 0);
    assert!(b.sites().is_empty());

    // A closes: its lease is free at once; B takes over and may write.
    lease_a.release();
    assert_eq!(lease_b.claim(1002), Role::Maintainer);
    b.set_reader_of(None);
    b.add_site("Chantier", vec![files.path().to_string_lossy().into_owned()]).unwrap();
}

#[test]
fn personal_files_leave_the_data_folder_once() {
    let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let personal = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    // An older version kept them next to the indexes.
    let old = Engine::open(data.path()).unwrap();
    old.save_search("Factures", "facture", serde_json::json!({})).unwrap();
    drop(old);
    let engine = Engine::open_with(data.path(), personal.path()).unwrap();
    assert_eq!(engine.saved_searches().len(), 1);
    assert!(!data.path().join("saved-searches.json").exists());
    assert!(personal.path().join("saved-searches.json").exists());
}
