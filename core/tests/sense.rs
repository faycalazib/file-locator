//! Étape 8, lot 8.1: the real meaning module (built by
//! `python scripts/sense-module.py` into target/sense-module). Without it,
//! these tests say so and pass: the module is optional.

use std::path::PathBuf;
use std::time::Instant;

use prospector_core::sense::{similarity, SenseModel, DIM};

fn module() -> Option<PathBuf> {
    let dir = std::env::var_os("PROSPECTOR_SENSE_MODULE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("target").join("sense-module").join("granite-97m-r2"));
    if dir.join("model.onnx").exists() {
        Some(dir)
    } else {
        eprintln!("meaning module not built (python scripts/sense-module.py): skipped");
        None
    }
}

#[test]
fn the_same_meaning_in_four_languages() {
    let Some(dir) = module() else { return };
    let started = Instant::now();
    let model = SenseModel::load(&dir).unwrap();
    eprintln!("loaded in {} ms", started.elapsed().as_millis());

    let texts = [
        "contrat de location d'un appartement",      // 0 fr
        "apartment lease agreement",                  // 1 en
        "contrato de arrendamiento de un piso",       // 2 es
        "عقد إيجار شقة",                              // 3 ar
        "recette de la tarte aux pommes",             // 4 fr, unrelated
        "apple pie recipe",                           // 5 en, unrelated
    ];
    let started = Instant::now();
    let v = model.embed(&texts).unwrap();
    eprintln!("6 texts in {} ms", started.elapsed().as_millis());
    assert_eq!(v.len(), 6);
    for x in &v {
        assert_eq!(x.len(), DIM);
        assert!((similarity(x, x) - 1.0).abs() < 1e-3, "normalized");
    }
    let s = |a: usize, b: usize| similarity(&v[a], &v[b]);
    for (a, b) in [(0, 1), (0, 2), (0, 3), (1, 3)] {
        eprintln!("{} ↔ {}: {:.3} (vs pie {:.3})", texts[a], texts[b], s(a, b), s(a, 5));
        assert!(s(a, b) > s(a, 5) + 0.1, "{} ↔ {}", texts[a], texts[b]);
    }
    assert!(s(4, 5) > s(4, 1) + 0.1, "the two recipes are closer to each other");

    // A batch gives the same vectors as one by one (padding does not matter).
    for i in 0..texts.len() {
        let alone = model.embed(&[texts[i]]).unwrap();
        assert!(similarity(&alone[0], &v[i]) > 0.9999, "{}", texts[i]);
    }
    // A long text is cut at the model's limit, not refused.
    let long = "clause de résiliation ".repeat(2000);
    assert_eq!(model.embed(&[long.as_str()]).unwrap().len(), 1);
}

/// The real ZIP: installed with its published SHA-256, then usable.
#[test]
fn the_real_module_installs_and_loads() {
    use prospector_core::sense::module::{install_zip, module_dir, status, MODULE_SHA256};
    let zip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("target").join("sense-module").join("prospector-sense-1.zip");
    if !zip.exists() {
        eprintln!("meaning module ZIP not built (python scripts/sense-module.py): skipped");
        return;
    }
    let personal = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let dir = module_dir(personal.path());
    let manifest = install_zip(&zip, &dir, Some(MODULE_SHA256)).unwrap();
    assert_eq!(manifest.model, "granite-embedding-97m-multilingual-r2");
    let s = status(&dir);
    assert!(s.installed && s.size > 100 * 1024 * 1024, "{s:?}");
    let model = SenseModel::load(&dir).unwrap();
    assert_eq!(model.embed(&["bail commercial"]).unwrap().len(), 1);
}

/// Throughput on this PC (run on demand: `cargo test --release --test sense -- --ignored --nocapture`).
#[test]
#[ignore]
fn throughput_of_passages() {
    let Some(dir) = module() else { return };
    let model = SenseModel::load(&dir).unwrap();
    let sentences = [
        "Le présent contrat de prestation prend effet à la date de signature par les deux parties.",
        "The supplier shall deliver the goods within thirty days of the purchase order.",
        "Las facturas son pagaderas a treinta días a partir de su fecha de emisión.",
        "يلتزم المستأجر بدفع الإيجار في بداية كل شهر ميلادي.",
        "En cas de manquement grave, le contrat peut être résilié de plein droit après mise en demeure.",
        "Annual maintenance includes two visits and the replacement of worn parts.",
    ];
    // Passages of about 250 words, all a little different (as real ones).
    let passages: Vec<String> = (0..120)
        .map(|i| (0..30 + i % 7).map(|j| sentences[(i + j) % sentences.len()]).collect::<Vec<_>>().join(" "))
        .collect();
    let refs: Vec<&str> = passages.iter().map(String::as_str).collect();
    model.embed(&refs[..4]).unwrap(); // warm-up
    for chunk in [1usize, 8, 32] {
        let started = Instant::now();
        for group in refs.chunks(chunk) {
            model.embed(group).unwrap();
        }
        let secs = started.elapsed().as_secs_f64();
        eprintln!("groups of {chunk:>2}: {:.1} passages/s", refs.len() as f64 / secs);
    }
    eprintln!("cores: {:?}", std::thread::available_parallelism());
}

/// What makes it faster (on demand): threads per session, sessions in
/// parallel, passage length, quantized or full model.
#[test]
#[ignore]
fn throughput_experiments() {
    let Some(dir) = module() else { return };
    let sentence = "Le présent contrat de prestation prend effet à la date de signature par les deux parties et reste valable trois ans. ";
    let make = |words: usize| -> Vec<String> { (0..48).map(|i| sentence.repeat(words / 20 + 1).chars().take(words * 6 + i).collect()).collect() };
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    let fp32 = std::env::var("PROSPECTOR_SENSE_FP32").ok();
    for (label, file) in [("int8", "model.onnx".to_owned())].into_iter().chain(fp32.map(|f| ("fp32", f))) {
        for words in [120usize, 250] {
            let passages = make(words);
            for (sessions, threads) in [(1, cores - 1), (2, (cores - 1) / 2), (4, (cores - 1) / 4), (8, 1.max((cores - 1) / 8))] {
                let models: Vec<SenseModel> = (0..sessions).map(|_| SenseModel::load_with(&dir, &file, threads).unwrap()).collect();
                models[0].embed(&[passages[0].as_str()]).unwrap();
                let started = Instant::now();
                std::thread::scope(|scope| {
                    for (k, model) in models.iter().enumerate() {
                        let mine: Vec<&str> = passages.iter().skip(k).step_by(sessions).map(String::as_str).collect();
                        scope.spawn(move || {
                            for p in mine {
                                model.embed(&[p]).unwrap();
                            }
                        });
                    }
                });
                let rate = passages.len() as f64 / started.elapsed().as_secs_f64();
                eprintln!("{label} ~{words} words: {sessions} × {threads} threads → {rate:.1} passages/s");
            }
        }
    }
}

/// Language bias (on demand): the same meaning in another language against
/// another meaning in the query's language.
#[test]
#[ignore]
fn language_bias() {
    let Some(dir) = module() else { return };
    let model = SenseModel::load(&dir).unwrap();
    let q = "contrat de prestation de services entre deux sociétés";
    let docs = [
        "contrato_servicios.docx\nEl presente contrato de prestación de servicios se regirá por la legislación española. La rescisión del contrato deberá notificarse con treinta días de antelación.",
        "contrat-prestation.docx\nLe présent contrat de prestation prend effet à la date de signature par les deux parties. Toute résiliation anticipée du contrat entraîne le versement d'une indemnité.",
        "reunion-2.txt\nCompte rendu de la réunion d'équipe numéro 2 : point sur le planning et les congés.",
        "service-agreement.pdf\nThis service agreement is entered into between the provider and the client.",
    ];
    let v = model.embed(&[q, docs[0], docs[1], docs[2], docs[3]]).unwrap();
    for (i, d) in docs.iter().enumerate() {
        eprintln!("{:.3}  {}", similarity(&v[0], &v[i + 1]), d.lines().next().unwrap());
    }
    let body = |d: &'static str| &d[d.find('\n').unwrap() + 1..];
    let bare = model.embed(&[q, body(docs[0]), body(docs[2])]).unwrap();
    eprintln!("without file names: es {:.3}, reunion {:.3}", similarity(&bare[0], &bare[1]), similarity(&bare[0], &bare[2]));
}
