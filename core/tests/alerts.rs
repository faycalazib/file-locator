//! Lot 6.1: alerts on a saved search. The files that start matching after an
//! update are announced, once each.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use prospector_core::{Engine, SearchRequest};

fn names(paths: &[String]) -> Vec<String> {
    let mut out: Vec<String> = paths.iter().map(|p| Path::new(p).file_name().unwrap().to_string_lossy().into_owned()).collect();
    out.sort();
    out
}

fn update(engine: &Engine, id: &str, paths: &[PathBuf]) {
    engine.update_paths(id, paths, &AtomicBool::new(false), &|_| {}).unwrap();
}

#[test]
fn files_that_start_matching_are_announced_once() {
    let files = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let root = files.path();
    std::fs::write(root.join("ancien.txt"), "Un contrat signé l'an dernier.").unwrap();
    std::fs::write(root.join("liste.txt"), "Rien à voir : des courses.").unwrap();

    let engine = Engine::open(data.path()).unwrap();
    let site = engine.add_site("Alertes", vec![root.to_string_lossy().into_owned()]).unwrap();
    engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    // The first indexing is not news for an alert made afterwards.
    let _ = engine.check_alerts(&site.id).unwrap();

    let saved = engine.save_search("Contrats", "contrat", serde_json::json!({})).unwrap();
    let request = SearchRequest { query: "contrat".into(), ..Default::default() };
    let saved = engine.enable_alert(&saved.id, request, vec![site.id.clone()]).unwrap();
    let alert = saved.alert.as_ref().unwrap();
    assert_eq!(names(&alert.seen.iter().cloned().collect::<Vec<_>>()), ["ancien.txt"], "already matching: remembered");

    // A new matching file, a file that now matches, a file that does not.
    std::fs::write(root.join("nouveau.txt"), "Le nouveau contrat de Dupont.").unwrap();
    std::fs::write(root.join("liste.txt"), "Courses, puis relire le contrat.").unwrap();
    std::fs::write(root.join("autre.txt"), "Des vacances.").unwrap();
    update(&engine, &site.id, &[root.join("nouveau.txt"), root.join("liste.txt"), root.join("autre.txt")]);
    let news = engine.check_alerts(&site.id).unwrap();
    assert_eq!(news.len(), 1);
    assert_eq!(news[0].label, "Contrats");
    assert_eq!(names(&news[0].found), ["liste.txt", "nouveau.txt"]);

    // Saved again, still matching: not news. The old file changed: not news.
    std::fs::write(root.join("nouveau.txt"), "Le nouveau contrat de Dupont, relu.").unwrap();
    std::fs::write(root.join("ancien.txt"), "Un contrat signé l'an dernier, annoté.").unwrap();
    update(&engine, &site.id, &[root.join("nouveau.txt"), root.join("ancien.txt")]);
    assert!(engine.check_alerts(&site.id).unwrap().is_empty());

    // The badge: fresh until read.
    let listed = engine.saved_searches().into_iter().find(|s| s.id == saved.id).unwrap();
    assert_eq!(names(&listed.alert.unwrap().fresh), ["liste.txt", "nouveau.txt"]);
    let read = engine.mark_alert_read(&saved.id).unwrap();
    assert!(read.alert.unwrap().fresh.is_empty());

    // Deleted then written again: announced again.
    std::fs::remove_file(root.join("nouveau.txt")).unwrap();
    update(&engine, &site.id, &[root.join("nouveau.txt")]);
    assert!(engine.check_alerts(&site.id).unwrap().is_empty());
    std::fs::write(root.join("nouveau.txt"), "Le contrat revient.").unwrap();
    update(&engine, &site.id, &[root.join("nouveau.txt")]);
    assert_eq!(names(&engine.check_alerts(&site.id).unwrap()[0].found), ["nouveau.txt"]);

    // Off: nothing more.
    engine.disable_alert(&saved.id).unwrap();
    std::fs::write(root.join("encore.txt"), "Encore un contrat.").unwrap();
    update(&engine, &site.id, &[root.join("encore.txt")]);
    assert!(engine.check_alerts(&site.id).unwrap().is_empty());
}
