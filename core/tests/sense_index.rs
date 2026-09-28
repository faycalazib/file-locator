//! Lot 8.2: the meaning index of a real site, with the real module (built
//! by `python scripts/sense-module.py`; without it, skipped). A question in
//! one language finds documents in the others; source code is left out; an
//! interrupted computation resumes without duplicates; only changed files
//! are computed again.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use prospector_core::sense::pool::{Pace, SensePool};
use prospector_core::{Engine, SenseProgress};

fn module() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("target").join("sense-module").join("granite-97m-r2");
    if dir.join("model.onnx").exists() {
        Some(dir)
    } else {
        eprintln!("meaning module not built (python scripts/sense-module.py): skipped");
        None
    }
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("test_fixtures")
}

fn names(engine: &Engine, pool: &SensePool, ids: &[String], question: &str) -> Vec<String> {
    let q = pool.embed(&[question.to_owned()]).unwrap()[0];
    let mut out: Vec<String> = Vec::new();
    for hit in engine.sense_search(ids, &q, 20).unwrap() {
        let name = Path::new(hit.hit.path.as_str()).file_name().unwrap().to_string_lossy().into_owned();
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

#[test]
fn a_site_understood_in_four_languages() {
    let Some(dir) = module() else { return };
    let files = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    for (from, to) in [
        ("fr/contrat-prestation.docx", "contrat-prestation.docx"),
        ("es/contrato_servicios.docx", "contrato_servicios.docx"),
        ("en/service-agreement.pdf", "service-agreement.pdf"),
        ("ar/عقد_إيجار.docx", "عقد_إيجار.docx"),
        ("fr/factures-2024.xlsx", "factures-2024.xlsx"),
        ("code/ContractService.ts", "ContractService.ts"),
    ] {
        std::fs::copy(fixtures().join(from), files.path().join(to)).unwrap();
    }
    std::fs::write(files.path().join("recette.txt"), "Pour la tarte aux pommes, étalez la pâte, disposez les fruits et enfournez trente minutes.").unwrap();
    // Enough files for several batches (the stop must fall between two).
    for i in 0..40 {
        std::fs::write(files.path().join(format!("reunion-{i}.txt")), format!("Compte rendu de la réunion d'équipe numéro {i} : point sur le planning et les congés.")).unwrap();
    }

    let engine = Engine::open(data.path()).unwrap();
    let site = engine.add_site("Contrats", vec![files.path().to_string_lossy().into_owned()]).unwrap();
    engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    let ids = vec![site.id.clone()];
    engine.set_site_sense(&site.id, true).unwrap();

    let plan = engine.sense_plan(&site.id, true).unwrap();
    assert_eq!(plan.files, 46, "everything but the code: {plan:?}");
    assert!(plan.passages >= 6, "{plan:?}");
    eprintln!("plan: {plan:?}");

    let pool = SensePool::load(&dir, Pace::Normal).unwrap();
    eprintln!("speed: {:.1} passages/s", pool.speed().unwrap());
    let embed = |texts: &[String]| pool.embed(texts);

    // Stopped after the first call: resumed later, no passage twice.
    let cancel = AtomicBool::new(false);
    let calls = AtomicUsize::new(0);
    let stop_early = |_: SenseProgress| {
        if calls.fetch_add(1, Ordering::Relaxed) == 1 {
            cancel.store(true, Ordering::Relaxed);
        }
    };
    let finished = engine.sense_update(&site.id, &embed, &cancel, &stop_early).unwrap();
    assert!(!finished, "stopped");
    assert!(engine.sense_pending(&site.id));
    assert!(engine.sense_update(&site.id, &embed, &AtomicBool::new(false), &|_| {}).unwrap());
    assert!(!engine.sense_pending(&site.id));
    let total = engine.sense_plan(&site.id, false).unwrap().done;
    assert_eq!(total as usize, plan.passages, "every passage once");

    // Across languages: an English question finds the Arabic lease first,
    // a French one the service contracts in three languages.
    let lease = names(&engine, &pool, &ids, "apartment rental agreement between landlord and tenant");
    eprintln!("lease: {lease:?}");
    assert_eq!(lease[0], "عقد_إيجار.docx");
    let q = pool.embed(&["contrat de prestation de services entre deux sociétés".to_owned()]).unwrap()[0];
    for hit in engine.sense_search(&ids, &q, 6).unwrap() {
        eprintln!("{:.3} {} #{}", hit.hit.score, hit.hit.path, hit.hit.passage);
    }
    let service = names(&engine, &pool, &ids, "contrat de prestation de services entre deux sociétés");
    eprintln!("service: {service:?}");
    for name in ["contrat-prestation.docx", "contrato_servicios.docx", "service-agreement.pdf"] {
        // Scores are close (0.77 to 0.80): a document in the query's language
        // wins a tie; the Spanish one also speaks of archiving (5th of 46).
        assert!(service[..5].contains(&name.to_owned()), "{name} in {service:?}");
    }
    let pie = names(&engine, &pool, &ids, "apple pie recipe");
    assert_eq!(pie[0], "recette.txt");
    // Source code is never understood.
    assert!(!names(&engine, &pool, &ids, "contract service class").contains(&"ContractService.ts".to_owned()));

    // A changed file: only it is computed again.
    std::fs::write(files.path().join("recette.txt"), "Pour la ratatouille, coupez les courgettes, les aubergines et les poivrons.").unwrap();
    engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    let again = engine.sense_plan(&site.id, true).unwrap();
    assert_eq!((again.files, again.passages), (1, 1), "{again:?}");
    engine.sense_update(&site.id, &embed, &AtomicBool::new(false), &|_| {}).unwrap();
    assert_eq!(names(&engine, &pool, &ids, "ratatouille aux légumes")[0], "recette.txt");
    assert_eq!(engine.sense_plan(&site.id, false).unwrap().done, total);

    // Turned off: the meaning index is deleted.
    engine.set_site_sense(&site.id, false).unwrap();
    assert!(engine.sense_search(&ids, &pool.embed(&["bail".to_owned()]).unwrap()[0], 5).unwrap().is_empty());
}

/// The whole chain on the 100,000 files of the performance corpus (run on
/// demand: `cargo test --release --test sense_index -- --ignored --nocapture`):
/// reading the main index, batches, commits, and the search among all vectors.
#[test]
#[ignore]
fn meaning_of_100_000_files() {
    use std::time::Instant;
    let Some(dir) = module() else { return };
    let corpus = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("target").join("tmp").join("perf-corpus");
    if !corpus.is_dir() {
        eprintln!("no performance corpus: skipped");
        return;
    }
    let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let engine = Engine::open(data.path()).unwrap();
    let site = engine.add_site("Perf", vec![corpus.to_string_lossy().into_owned()]).unwrap();
    let started = Instant::now();
    engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    eprintln!("indexing: {:.0} s", started.elapsed().as_secs_f64());
    engine.set_site_sense(&site.id, true).unwrap();
    let started = Instant::now();
    let plan = engine.sense_plan(&site.id, true).unwrap();
    eprintln!("estimate: {plan:?} in {:.1} s", started.elapsed().as_secs_f64());
    let pool = SensePool::load(&dir, Pace::Normal).unwrap();
    let speed = pool.speed().unwrap();
    eprintln!("speed: {speed:.1} passages/s → estimated {:.0} min", plan.passages as f32 / speed / 60.0);
    let started = Instant::now();
    let last = std::sync::Mutex::new(Instant::now());
    engine
        .sense_update(&site.id, &|t: &[String]| pool.embed(t), &AtomicBool::new(false), &|p| {
            let mut last = last.lock().unwrap();
            if last.elapsed().as_secs() >= 60 {
                *last = Instant::now();
                eprintln!("  {} / {} passages, {:.0} s", p.done, p.total, started.elapsed().as_secs_f64());
            }
        })
        .unwrap();
    let secs = started.elapsed().as_secs_f64();
    eprintln!("meaning: {} passages in {:.0} min ({:.1}/s)", plan.passages, secs / 60.0, plan.passages as f64 / secs);
    let q = pool.embed(&["planning des congés de l'équipe".to_owned()]).unwrap()[0];
    let started = Instant::now();
    let hits = engine.sense_search(std::slice::from_ref(&site.id), &q, 50).unwrap();
    eprintln!("search among {} vectors: {} ms", engine.sense_passages(&site.id), started.elapsed().as_millis());
    assert_eq!(hits.len(), 50);
    let size: u64 = walk(&data.path().join("indexes").join(format!("{}.sense", site.id)));
    eprintln!("meaning index on disk: {:.1} MB", size as f64 / 1024.0 / 1024.0);
}

/// Lot 8.3: words and meaning together, the filters applied to both.
#[test]
fn search_by_words_and_by_meaning() {
    let Some(dir) = module() else { return };
    let files = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    for (from, to) in [
        ("fr/contrat-prestation.docx", "contrat-prestation.docx"),
        ("es/contrato_servicios.docx", "contrato_servicios.docx"),
        ("en/service-agreement.pdf", "service-agreement.pdf"),
        ("ar/عقد_إيجار.docx", "عقد_إيجار.docx"),
    ] {
        std::fs::copy(fixtures().join(from), files.path().join(to)).unwrap();
    }
    std::fs::write(files.path().join("recette.txt"), "Pour la tarte aux pommes, étalez la pâte, disposez les fruits et enfournez trente minutes.").unwrap();
    let engine = Engine::open(data.path()).unwrap();
    let site = engine.add_site("Contrats", vec![files.path().to_string_lossy().into_owned()]).unwrap();
    engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    engine.set_site_sense(&site.id, true).unwrap();
    let pool = SensePool::load(&dir, Pace::Normal).unwrap();
    engine.sense_update(&site.id, &|t: &[String]| pool.embed(t), &AtomicBool::new(false), &|_| {}).unwrap();
    let ids = vec![site.id.clone()];
    let ask = |query: &str, kinds: &[&str]| {
        let question = pool.embed(&[prospector_core::sense::question_text(query)]).unwrap()[0];
        let request = prospector_core::SearchRequest { query: query.into(), kinds: kinds.iter().map(|k| (*k).to_owned()).collect(), ..Default::default() };
        engine.search_meaning(&ids, &request, &question).unwrap()
    };
    let name = |h: &prospector_core::Hit| Path::new(h.path.as_str()).file_name().unwrap().to_string_lossy().into_owned();

    // No French word of the search in the Arabic lease: found by meaning only, with its passage.
    let lease = ask("bail d'habitation entre propriétaire et locataire", &[]);
    for h in &lease.hits {
        eprintln!("{} {:?}", name(h), h.meaning.as_ref().map(|m| (m.score, m.only)));
    }
    let arabic = lease.hits.iter().find(|h| name(h) == "عقد_إيجار.docx").expect("the Arabic lease");
    let meaning = arabic.meaning.as_ref().unwrap();
    assert!(meaning.only && arabic.match_count == 0);
    assert!(!arabic.snippets.is_empty() && !arabic.snippets[0].text.is_empty());
    assert!(lease.hits.iter().all(|h| name(h) != "recette.txt"), "the recipe is too far");
    // The preview opens on that passage.
    let preview = engine.preview_passage(&site.id, &arabic.path, &prospector_core::SearchRequest::default(), Some((meaning.start, meaning.end))).unwrap();
    assert_eq!(preview.match_count, 1);

    // Filters apply to the documents found by meaning too.
    assert!(ask("bail d'habitation entre propriétaire et locataire", &["pdf"]).hits.iter().all(|h| name(h) != "عقد_إيجار.docx"));

    // Words and meaning: the French contract has both and comes first.
    let both = ask("contrat de prestation", &[]);
    assert_eq!(name(&both.hits[0]), "contrat-prestation.docx");
    assert!(both.hits[0].meaning.as_ref().is_some_and(|m| !m.only) && both.hits[0].match_count > 0);
}

/// Lot 8.4: a shared index. The PC holding it computes the meaning; a
/// reader searches it by meaning, and cannot tick "Meaning" itself.
#[test]
fn a_reader_of_a_shared_index_searches_by_meaning() {
    let Some(dir) = module() else { return };
    let files = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let shared = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let (me_a, me_b) = (tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap(), tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap());
    std::fs::copy(fixtures().join("ar/عقد_إيجار.docx"), files.path().join("عقد_إيجار.docx")).unwrap();
    std::fs::copy(fixtures().join("fr/contrat-prestation.docx"), files.path().join("contrat-prestation.docx")).unwrap();

    let a = Engine::open_with(shared.path(), me_a.path()).unwrap();
    let site = a.add_site("Baux", vec![files.path().to_string_lossy().into_owned()]).unwrap();
    a.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    a.set_site_sense(&site.id, true).unwrap();
    let pool = SensePool::load(&dir, Pace::Economy).unwrap();
    a.sense_update(&site.id, &|t: &[String]| pool.embed(t), &AtomicBool::new(false), &|_| {}).unwrap();

    let b = Engine::open_with(shared.path(), me_b.path()).unwrap();
    b.set_reader_of(Some("BUREAU".into()));
    let question = pool.embed(&["apartment lease".to_owned()]).unwrap()[0];
    let request = prospector_core::SearchRequest { query: "apartment lease".into(), ..Default::default() };
    let found = b.search_meaning(std::slice::from_ref(&site.id), &request, &question).unwrap();
    assert!(found.hits.iter().any(|h| h.path.ends_with("عقد_إيجار.docx") && h.meaning.as_ref().is_some_and(|m| m.only)));
    assert!(b.set_site_sense(&site.id, false).is_err(), "only the holder changes it");
}

fn walk(dir: &Path) -> u64 {
    std::fs::read_dir(dir).map(|entries| entries.flatten().map(|e| if e.path().is_dir() { walk(&e.path()) } else { e.metadata().map(|m| m.len()).unwrap_or(0) }).sum()).unwrap_or(0)
}
