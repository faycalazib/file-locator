//! Lot 6.8: a USB drive that got another letter. The paths of a real indexed
//! site are moved to another letter and back, with nothing read again.

use std::sync::atomic::AtomicBool;

use prospector_core::portable::{drive_of, move_drive, DriveMove};
use prospector_core::{Engine, SearchRequest};

fn search(engine: &Engine, id: &str, query: &str) -> Vec<String> {
    let request = SearchRequest { query: query.into(), ..Default::default() };
    let mut paths: Vec<String> = engine.search(&[id.to_owned()], &request).unwrap().hits.into_iter().map(|h| h.path).collect();
    paths.sort();
    paths
}

#[test]
fn a_new_drive_letter_moves_sites_indexes_and_saved_searches() {
    let files = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let root = files.path();
    std::fs::write(root.join("devis.txt"), "Devis de charpente pour la toiture.").unwrap();
    let zip = std::fs::File::create(root.join("annexes.zip")).unwrap();
    let mut writer = zip::ZipWriter::new(zip);
    writer.start_file("plans/notice.txt", zip::write::SimpleFileOptions::default()).unwrap();
    std::io::Write::write_all(&mut writer, "Notice de la charpente.".as_bytes()).unwrap();
    writer.finish().unwrap();

    let here = drive_of(root).expect("a drive letter");
    let other = if here == "Q:" { "R:" } else { "Q:" };
    let (id, before) = {
        let engine = Engine::open(data.path()).unwrap();
        let site = engine.add_site("Clé", vec![root.to_string_lossy().into_owned()]).unwrap();
        engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
        engine.save_search("Charpente", "charpente", serde_json::json!({ "folder": root.to_string_lossy() })).unwrap();
        let before = search(&engine, &site.id, "charpente");
        assert_eq!(before.len(), 2, "{before:?}");
        (site.id, before)
    };

    // The drive is now `other`: everything follows, found at once.
    let moved = move_drive(data.path(), data.path(), &here, other).unwrap();
    assert_eq!(moved, DriveMove { sites: 1, documents: 2 });
    let engine = Engine::open(data.path()).unwrap();
    let site = engine.sites().into_iter().find(|s| s.id == id).unwrap();
    assert!(site.roots[0].starts_with(other), "{:?}", site.roots);
    let after = search(&engine, &id, "charpente");
    let expected: Vec<String> = before.iter().map(|p| format!("{other}{}", &p[here.len()..])).collect();
    assert_eq!(after, expected);
    let manifest = std::fs::read_to_string(data.path().join("indexes").join(format!("{id}.manifest.json"))).unwrap();
    assert!(!manifest.contains(&format!("\"{here}\\\\")) && manifest.contains(&format!("\"{other}\\\\")), "{manifest}");
    let saved = serde_json::to_string(&engine.saved_searches()).unwrap();
    assert!(saved.contains(&format!("{other}\\\\")), "{saved}");
    drop(engine);

    // Run again: nothing left to move. Then back to the real letter.
    assert_eq!(move_drive(data.path(), data.path(), &here, other).unwrap(), DriveMove::default());
    move_drive(data.path(), data.path(), other, &here).unwrap();
    let engine = Engine::open(data.path()).unwrap();
    assert_eq!(search(&engine, &id, "charpente"), before);
    // The manifest matches the files again: an update finds nothing to do.
    let site = engine.index_site(&id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    assert_eq!(site.doc_count, 2);
    assert_eq!(search(&engine, &id, "toiture").len(), 1);
}
