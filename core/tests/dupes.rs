//! Lot 6.5: identical files and near-identical documents.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use prospector_core::dupes::{DupKind, DupOptions};
use prospector_core::Engine;

fn long_text(topic: &str) -> String {
    let words = ["contrat", "prestation", "client", "paiement", "durée", "résiliation", "annexe", "tarif", "délai", "signature", "article", "obligation"];
    let mut out = format!("{topic}. ");
    for i in 0..400 {
        out.push_str(words[(i * 7 + topic.len()) % words.len()]);
        out.push_str(if i % 9 == 8 { ". " } else { " " });
        if i % 13 == 0 {
            out.push_str(&format!("{topic}{i} "));
        }
    }
    out
}

/// A minimal Word document holding `text`, one paragraph per sentence.
fn write_docx(path: &Path, text: &str) {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("[Content_Types].xml", options).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#).unwrap();
    zip.start_file("word/document.xml", options).unwrap();
    let paragraphs: String = text.split(". ").map(|p| format!("<w:p><w:r><w:t>{p}.</w:t></w:r></w:p>")).collect();
    let xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{paragraphs}</w:body></w:document>"#);
    zip.write_all(xml.as_bytes()).unwrap();
    zip.finish().unwrap();
}

fn names(files: &[prospector_core::dupes::DupFile]) -> Vec<String> {
    let mut out: Vec<String> = files.iter().map(|f| Path::new(&f.path).file_name().unwrap().to_string_lossy().into_owned()).collect();
    out.sort();
    out
}

#[test]
fn identical_and_near_identical_documents() {
    let files = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let root = files.path();
    let base = long_text("Location");
    std::fs::write(root.join("original.txt"), &base).unwrap();
    std::fs::write(root.join("copie de original.txt"), &base).unwrap();
    // Same size, other bytes: not identical.
    let mut same_size = base.clone().into_bytes();
    same_size[10] = b'Z';
    std::fs::write(root.join("meme-taille.txt"), &same_size).unwrap();
    // A few words changed: near-identical.
    std::fs::write(root.join("retouche.txt"), base.replacen("paiement", "règlement", 3)).unwrap();
    std::fs::write(root.join("autre.txt"), long_text("Inventaire annuel du stock de pièces")).unwrap();
    std::fs::write(root.join("vide-1.txt"), "").unwrap();
    std::fs::write(root.join("vide-2.txt"), "").unwrap();
    // The same text as plain text and as a Word document.
    let rapport = long_text("Rapport de chantier");
    std::fs::write(root.join("rapport.txt"), &rapport).unwrap();
    write_docx(&root.join("rapport.docx"), &rapport);

    let engine = Engine::open(data.path()).unwrap();
    let site = engine.add_site("Doublons", vec![root.to_string_lossy().into_owned()]).unwrap();
    engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
    let report = engine.find_duplicates(std::slice::from_ref(&site.id), DupOptions::default(), &AtomicBool::new(false), &|_| {}).unwrap();

    let exact: Vec<_> = report.groups.iter().filter(|g| g.kind == DupKind::Exact).collect();
    assert_eq!(exact.len(), 1, "{:?}", report.groups);
    assert_eq!(names(&exact[0].files), ["copie de original.txt", "original.txt"]);
    assert_eq!(exact[0].wasted, base.len() as u64);

    let similar: Vec<_> = report.groups.iter().filter(|g| g.kind == DupKind::Similar).collect();
    let with_retouche = similar.iter().find(|g| names(&g.files).contains(&"retouche.txt".to_owned())).expect("retouched text found");
    assert!(names(&with_retouche.files).contains(&"meme-taille.txt".to_owned()) || names(&with_retouche.files).contains(&"original.txt".to_owned()));
    assert!(with_retouche.similarity >= 0.8);
    assert!(!similar.iter().any(|g| names(&g.files).contains(&"autre.txt".to_owned())), "a different text is not close");
    let formats = similar.iter().find(|g| names(&g.files).contains(&"rapport.docx".to_owned()));
    assert!(formats.is_some_and(|g| names(&g.files).contains(&"rapport.txt".to_owned())), "the text and Word versions: {similar:?}");

    // Identical only.
    let only_exact = engine
        .find_duplicates(std::slice::from_ref(&site.id), DupOptions { similar: false, ..Default::default() }, &AtomicBool::new(false), &|_| {})
        .unwrap();
    assert!(only_exact.groups.iter().all(|g| g.kind == DupKind::Exact));
}
