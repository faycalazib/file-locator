//! Performance check of goal.md §7: "indexed search < 200 ms on a 100k-file index".
//!
//! Ignored by default (it writes 100 000 files). Run it in release mode:
//!   cargo test -p prospector-core --release --test perf -- --ignored --nocapture
//! The corpus stays in target/tmp/perf-corpus and is reused by later runs.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use prospector_core::{Engine, SearchRequest};

const FILES: usize = 100_000;

const WORDS_FR: &[&str] = &["contrat", "facture", "résiliation", "prestation", "client", "paiement", "échéance", "annexe", "signature", "avenant", "livraison", "garantie"];
const WORDS_ES: &[&str] = &["contrato", "factura", "rescisión", "servicio", "cliente", "pago", "vencimiento", "anexo", "firma", "entrega", "garantía"];
const WORDS_EN: &[&str] = &["agreement", "invoice", "termination", "service", "customer", "payment", "deadline", "appendix", "delivery", "warranty"];
const WORDS_AR: &[&str] = &["عقد", "فاتورة", "فسخ", "خدمة", "عميل", "دفع", "موعد", "ملحق", "توقيع", "تسليم", "ضمان"];

/// Deterministic pseudo-random generator (no dependency).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as usize
    }
}

fn corpus() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("perf-corpus");
    let marker = dir.join(".complete");
    if marker.exists() {
        return dir;
    }
    let started = Instant::now();
    let mut rng = Lcg(42);
    for i in 0..FILES {
        let words = match i % 4 {
            0 => WORDS_FR,
            1 => WORDS_ES,
            2 => WORDS_EN,
            _ => WORDS_AR,
        };
        let sub = dir.join(format!("{:03}", i / 1000));
        if i % 1000 == 0 {
            fs::create_dir_all(&sub).unwrap();
        }
        let text: Vec<&str> = (0..120).map(|_| words[rng.next() % words.len()]).collect();
        fs::write(sub.join(format!("doc-{i:06}.txt")), text.join(" ")).unwrap();
    }
    fs::write(marker, "").unwrap();
    println!("corpus: {FILES} files written in {:.1?}", started.elapsed());
    dir
}

#[test]
#[ignore = "writes 100k files; run in release with --ignored"]
fn search_under_200ms_on_100k_files() {
    let corpus = corpus();
    // PROSPECTOR_DIAG_DIR: write the index on another disk (antivirus comparisons).
    let base = std::env::var("PROSPECTOR_DIAG_DIR").unwrap_or_else(|_| env!("CARGO_TARGET_TMPDIR").to_owned());
    let data = tempfile::tempdir_in(base).unwrap();
    let engine = Engine::open(data.path()).unwrap();
    let site = engine.add_site("Perf", vec![corpus.to_string_lossy().into_owned()]).unwrap();

    let started = Instant::now();
    let site = engine
        .index_site(&site.id, &[], &AtomicBool::new(false), &|p| {
            if p.done % 10_000 == 0 {
                println!("  {:>6.1?} {:?} {}/{}", started.elapsed(), p.phase, p.done, p.total);
            }
        })
        .unwrap();
    println!("indexing: {} files in {:.1?}", site.doc_count, started.elapsed());
    assert_eq!(site.doc_count as usize, FILES);

    let queries = [
        ("contrat", false),
        ("contratos", false),
        ("عقد", false),
        ("resiliation", false),
        ("contrat OR contrato OR عقد", false),
        ("\"contrat facture\"", false),
        ("paiemant", true),
        ("facture NOT annexe", false),
    ];
    let mut worst = 0u128;
    for (query, fuzzy) in queries {
        // First call warms the OS cache; the second one is measured.
        // As the UI asks it: with the counts per criterion (lot 6.3).
        let req = SearchRequest { query: query.into(), fuzzy, limit: Some(200), facets: true, ..Default::default() };
        engine.search(std::slice::from_ref(&site.id), &req).unwrap();
        let t = Instant::now();
        let response = engine.search(std::slice::from_ref(&site.id), &req).unwrap();
        let ms = t.elapsed().as_millis();
        worst = worst.max(ms);
        println!("{query:<32} fuzzy={fuzzy:<5} → {:>6} files, top {:>3} in {ms} ms", response.total_files, response.hits.len());
    }
    assert!(worst < 200, "slowest search took {worst} ms (target < 200 ms)");
}

/// Timings on a copy of a real data folder (PROSPECTOR_BENCH_DATA), where
/// code gives millions of distinct words: typo tolerance and "contains"
/// (`ab` off) walk that dictionary.
#[test]
#[ignore = "diagnostic: needs PROSPECTOR_BENCH_DATA"]
fn real_index_timings() {
    let Ok(data) = std::env::var("PROSPECTOR_BENCH_DATA") else { return };
    let engine = Engine::open(std::path::Path::new(&data)).unwrap();
    let ids: Vec<String> = engine.sites().into_iter().map(|s| s.id).collect();
    let cases: [(&str, bool, bool, bool); 8] = [
        // `Aa` on a frequent word: every candidate is checked on its text.
        ("Import", true, true, true),
        ("Contrat", true, false, true),
        // query, fuzzy, whole word, case-sensitive
        ("contrat", false, true, false),
        ("contrat", true, true, false),
        ("contrat", false, false, false),
        ("Contrat", true, true, true),
        ("import", true, true, false),
        ("paiemant", true, true, false),
    ];
    for (query, fuzzy, whole_word, case_sensitive) in cases {
        for (facets, limit) in [(false, 200), (true, 200), (true, 20)] {
            let req = SearchRequest {
                query: query.into(),
                fuzzy,
                whole_word,
                case_sensitive,
                limit: Some(limit),
                facets,
                ..Default::default()
            };
            engine.search(&ids, &req).unwrap();
            let t = Instant::now();
            let response = engine.search(&ids, &req).unwrap();
            let kinds: usize = response.facets.as_ref().map_or(0, |f| f.kinds.values().sum());
            println!(
                "{query:<10} fuzzy={fuzzy:<5} word={whole_word:<5} Aa={case_sensitive:<5} facets={facets:<5} top={limit:<3} → {:>5} files (kinds {kinds:>5}) in {:>5} ms",
                response.total_files,
                t.elapsed().as_millis()
            );
        }
    }
}
