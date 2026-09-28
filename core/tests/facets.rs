//! Lot 6.3: counts per kind, language, year and site, over everything found,
//! each criterion counted without its own filter.

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, UNIX_EPOCH};

use prospector_core::{Engine, SearchRequest};

/// A text file dated `secs` (Unix time).
fn write(path: &Path, text: &str, secs: u64) {
    std::fs::write(path, text).unwrap();
    let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(UNIX_EPOCH + Duration::from_secs(secs)).unwrap();
}

const Y2023: u64 = 1_690_000_000; // July 2023
const Y2025: u64 = 1_750_000_000; // June 2025

fn count(map: &std::collections::BTreeMap<String, usize>, key: &str) -> usize {
    map.get(key).copied().unwrap_or(0)
}

#[test]
fn counts_per_criterion_without_their_own_filter() {
    let files = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let (a, b) = (files.path().join("a"), files.path().join("b"));
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    write(&a.join("contrat-1.txt"), "Le contrat de location est signé par les deux parties aujourd'hui.", Y2023);
    write(&a.join("contrat-2.txt"), "Ce contrat de travail prend effet le premier du mois prochain.", Y2025);
    write(&a.join("contrat.md"), "The contract (contrat in French) was signed by both parties today.", Y2025);
    write(&a.join("autre.txt"), "Rien à voir avec le sujet.", Y2025);
    write(&b.join("contrat-b.txt"), "Un autre contrat, rangé dans le second site de fouille.", Y2025);

    let engine = Engine::open(data.path()).unwrap();
    let site_a = engine.add_site("A", vec![a.to_string_lossy().into_owned()]).unwrap();
    let site_b = engine.add_site("B", vec![b.to_string_lossy().into_owned()]).unwrap();
    for site in [&site_a, &site_b] {
        engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    }
    let only_a = std::slice::from_ref(&site_a.id);
    let req = |query: &str| SearchRequest { query: query.into(), facets: true, fuzzy: false, ..Default::default() };

    let all = engine.search(only_a, &req("contrat")).unwrap();
    let facets = all.facets.expect("asked");
    assert_eq!(all.total_files, 3);
    // .txt and .md are both "text".
    assert_eq!(count(&facets.kinds, "text"), 3);
    assert_eq!(facets.kinds.values().sum::<usize>(), 3);
    assert_eq!(count(&facets.langs, "fr"), 2);
    assert_eq!(count(&facets.langs, "en"), 1);
    assert_eq!(count(&facets.years, "2023"), 1);
    assert_eq!(count(&facets.years, "2025"), 2);
    // Sites: the one searched, and the one not searched (what it would bring).
    assert_eq!(count(&facets.sites, &site_a.id), 3);
    assert_eq!(count(&facets.sites, &site_b.id), 1);

    // A language chosen: fewer hits, but every language is still counted.
    let french = engine.search(only_a, &SearchRequest { langs: vec!["fr".into()], ..req("contrat") }).unwrap();
    assert_eq!(french.hits.len(), 2);
    let facets = french.facets.unwrap();
    assert_eq!(count(&facets.langs, "en"), 1, "its own filter does not apply");
    assert_eq!(count(&facets.years, "2023") + count(&facets.years, "2025"), 2, "the other filters do");

    // A kind chosen that gives nothing: the kinds are still counted.
    let code = engine.search(only_a, &SearchRequest { kinds: vec!["code".into()], ..req("contrat") }).unwrap();
    assert!(code.hits.is_empty());
    assert_eq!(count(&code.facets.unwrap().kinds, "text"), 3);

    // A year chosen (2025): the other year is still counted.
    let jan_2025 = 1_735_689_600;
    let recent = engine.search(only_a, &SearchRequest { date_from: Some(jan_2025), ..req("contrat") }).unwrap();
    assert_eq!(recent.hits.len(), 2);
    assert_eq!(count(&recent.facets.unwrap().years, "2023"), 1);

    // A query checked on the text (regex): counted from the documents checked.
    let checked = engine.search(only_a, &req("/contra[ct]t?/")).unwrap();
    let facets = checked.facets.unwrap();
    assert_eq!(facets.kinds.values().sum::<usize>(), checked.total_files);
    assert_eq!(count(&facets.sites, &site_a.id), checked.total_files);

    // Not asked: not computed.
    assert!(engine.search(only_a, &SearchRequest { facets: false, ..req("contrat") }).unwrap().facets.is_none());
}
