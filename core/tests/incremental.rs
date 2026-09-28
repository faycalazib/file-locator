//! Étape 3: incremental indexing and folder watching. Each test works on its
//! own folder of small files, created in target/ (never in the system TEMP).

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

use prospector_core::index::SiteIndex;
use prospector_core::{Engine, FolderWatcher, IndexPhase, SearchRequest, SiteRecord};

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap()
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A site over `root` in a fresh data folder.
fn site(root: &Path) -> (Engine, SiteRecord, tempfile::TempDir) {
    let data = temp_dir();
    let engine = Engine::open(data.path()).unwrap();
    let site = engine.add_site("Test", vec![root.to_string_lossy().into_owned()]).unwrap();
    (engine, site, data)
}

/// Indexes the site; returns the record and the number of files read.
fn index(engine: &Engine, id: &str) -> (SiteRecord, usize) {
    let read = Mutex::new(0);
    let record = engine
        .index_site(id, &[], &AtomicBool::new(false), &|p| {
            if matches!(p.phase, IndexPhase::Reading) {
                *read.lock().unwrap() = p.total;
            }
        })
        .unwrap();
    (record, read.into_inner().unwrap())
}

fn update(engine: &Engine, id: &str, paths: &[PathBuf]) -> Option<SiteRecord> {
    engine.update_paths(id, paths, &AtomicBool::new(false), &|_| {}).unwrap()
}

/// File names (inner names for ZIP entries) found for `query`.
fn found(engine: &Engine, id: &str, query: &str) -> Vec<String> {
    let req = SearchRequest { query: query.into(), fuzzy: false, ..Default::default() };
    let mut names: Vec<String> = engine
        .search(&[id.to_owned()], &req)
        .unwrap()
        .hits
        .iter()
        .map(|h| h.path.rsplit(['\\', '/']).next().unwrap().to_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_second_update_reads_only_what_changed() {
    let dir = temp_dir();
    let root = dir.path();
    write(&root.join("a.txt"), "alpha original");
    write(&root.join("b.txt"), "bravo");
    write(&root.join("sub/c.txt"), "charlie");
    let (engine, site, _data) = site(root);

    let (record, read) = index(&engine, &site.id);
    assert_eq!((record.doc_count, read), (3, 3));

    // Nothing changed: nothing is read, nothing is lost.
    let (record, read) = index(&engine, &site.id);
    assert_eq!((record.doc_count, read), (3, 0));
    assert_eq!(found(&engine, &site.id, "bravo"), ["b.txt"]);

    // a.txt changed (size differs), b.txt removed, d.txt added.
    write(&root.join("a.txt"), "alpha rewritten entirely");
    std::fs::remove_file(root.join("b.txt")).unwrap();
    write(&root.join("d.txt"), "delta");
    let (record, read) = index(&engine, &site.id);
    assert_eq!((record.doc_count, read), (3, 2));
    assert!(found(&engine, &site.id, "original").is_empty(), "old content of a.txt is gone");
    assert_eq!(found(&engine, &site.id, "rewritten"), ["a.txt"]);
    assert!(found(&engine, &site.id, "bravo").is_empty(), "removed file left the index");
    assert_eq!(found(&engine, &site.id, "delta"), ["d.txt"]);
    assert_eq!(found(&engine, &site.id, "charlie"), ["c.txt"]);
}

#[test]
fn watcher_paths_update_files_folders_and_archives() {
    let dir = temp_dir();
    let root = dir.path();
    write(&root.join("keep.txt"), "kilo");
    write(&root.join("sub/c.txt"), "charlie");
    let zip = root.join("clients.zip");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_fixtures/archives/dossier-clients.zip");
    std::fs::copy(fixtures, &zip).unwrap();
    let (engine, site, _data) = site(root);
    let (record, _) = index(&engine, &site.id);
    let with_zip = record.doc_count;
    assert!(with_zip > 2, "the ZIP gives several documents");

    // A new file.
    write(&root.join("new.txt"), "november");
    let record = update(&engine, &site.id, &[root.join("new.txt")]).expect("changed");
    assert_eq!(record.doc_count, with_zip + 1);
    assert_eq!(found(&engine, &site.id, "november"), ["new.txt"]);

    // A removed folder takes its files with it.
    std::fs::remove_dir_all(root.join("sub")).unwrap();
    update(&engine, &site.id, &[root.join("sub")]).expect("changed");
    assert!(found(&engine, &site.id, "charlie").is_empty());

    // A folder that appears (copied or renamed) is read whole.
    write(&root.join("moved/deep/e.txt"), "echo");
    update(&engine, &site.id, &[root.join("moved")]).expect("changed");
    assert_eq!(found(&engine, &site.id, "echo"), ["e.txt"]);

    // Removing the ZIP removes every document it held.
    std::fs::remove_file(&zip).unwrap();
    let record = update(&engine, &site.id, &[zip]).expect("changed");
    assert_eq!(record.doc_count, 3, "keep + new + e");

    // Paths outside the site, hidden or excluded are ignored.
    write(&root.join(".git/HEAD"), "ref");
    write(&root.join("node_modules/x.js"), "x");
    let outside = temp_dir();
    write(&outside.path().join("o.txt"), "oscar");
    let ignored = [root.join(".git/HEAD"), root.join("node_modules/x.js"), outside.path().join("o.txt")];
    assert!(update(&engine, &site.id, &ignored).is_none());
    assert_eq!(found(&engine, &site.id, "kilo"), ["keep.txt"]);
}

#[test]
fn an_unplugged_root_keeps_its_documents() {
    let first = temp_dir();
    let second = temp_dir();
    write(&first.path().join("a.txt"), "alpha");
    write(&second.path().join("usb/b.txt"), "bravo");
    let usb = second.path().join("usb");
    let data = temp_dir();
    let engine = Engine::open(data.path()).unwrap();
    let roots = vec![first.path().to_string_lossy().into_owned(), usb.to_string_lossy().into_owned()];
    let site = engine.add_site("Two roots", roots).unwrap();
    assert_eq!(index(&engine, &site.id).0.doc_count, 2);

    // The disk is "unplugged": its root disappears.
    std::fs::rename(&usb, second.path().join("elsewhere")).unwrap();
    let (record, _) = index(&engine, &site.id);
    assert_eq!(record.doc_count, 2);
    assert_eq!(record.missing_roots.len(), 1);
    assert_eq!(found(&engine, &site.id, "bravo"), ["b.txt"]);
}

#[test]
fn an_index_from_an_older_version_is_rebuilt() {
    let dir = temp_dir();
    write(&dir.path().join("a.txt"), "alpha");
    let (engine, site, data) = site(dir.path());
    // An index with another schema where the site's index goes.
    let old = data.path().join("indexes").join(&site.id);
    std::fs::create_dir_all(&old).unwrap();
    let mut schema = tantivy::schema::Schema::builder();
    schema.add_text_field("path", tantivy::schema::STRING);
    drop(tantivy::Index::create_in_dir(&old, schema.build()).unwrap());

    assert!(SiteIndex::open_or_create(&old).is_ok(), "the old index is replaced");
    let (record, read) = index(&engine, &site.id);
    assert_eq!((record.doc_count, read), (1, 1));
    assert_eq!(found(&engine, &site.id, "alpha"), ["a.txt"]);
}

#[test]
fn the_watcher_reports_changed_paths() {
    let dir = temp_dir();
    let root = dir.path().canonicalize().unwrap();
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);
    let _watcher = FolderWatcher::start(std::slice::from_ref(&root), Duration::from_millis(200), move |changes| {
        let _ = tx.lock().unwrap().send(changes);
    })
    .expect("the folder can be watched");

    let file = root.join("w.txt");
    write(&file, "whiskey");
    let mut seen = Vec::new();
    while let Ok(changes) = rx.recv_timeout(Duration::from_secs(10)) {
        seen.extend(changes.paths);
        if seen.contains(&file) {
            return;
        }
    }
    panic!("no event for {file:?}, got {seen:?}");
}

#[test]
fn saved_searches_persist_and_follow_the_data_folder() {
    let data = temp_dir();
    let settings = serde_json::json!({ "mode": "indexed", "options": { "fuzzy": false } });
    {
        let engine = Engine::open(data.path()).unwrap();
        let first = engine.save_search("Factures", "facture", settings.clone()).unwrap();
        // Same query and settings: renamed, not duplicated.
        let again = engine.save_search("Factures 2024", "facture", settings.clone()).unwrap();
        assert_eq!(first.id, again.id);
        // Empty label: the query is the label.
        engine.save_search("  ", "/TODO|FIXME/", serde_json::Value::Null).unwrap();
        assert_eq!(engine.saved_searches().len(), 2);
    }
    let engine = Engine::open(data.path()).unwrap();
    let saved = engine.saved_searches();
    assert_eq!(saved.iter().map(|s| s.label.as_str()).collect::<Vec<_>>(), ["Factures 2024", "/TODO|FIXME/"]);
    assert_eq!(saved[0].settings, settings);
    engine.remove_saved_search(&saved[0].id).unwrap();
    engine.remove_saved_search("gone").unwrap();
    assert_eq!(Engine::open(data.path()).unwrap().saved_searches().len(), 1);
}
