//! Writes the 7z fixture of `test_fixtures/` (called by scripts/make-fixtures.py):
//! `cargo run -p prospector-core --example make_7z -- <output.7z>`.
//!
//! One **solid** LZMA2 block, with a binary entry between two documents: the
//! reader must read through the skipped entry to decode the next one.

use std::time::{Duration, UNIX_EPOCH};

use sevenz_rust2::{ArchiveEntry, ArchiveWriter, NtTime, SourceReader};

fn main() {
    let out = std::env::args().nth(1).expect("usage: make_7z <output.7z>");
    // Deterministic "photo": not a document, skipped by Prospector.
    let photo: Vec<u8> = (0..200_000u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8).collect();
    let entries: [(&str, Vec<u8>); 3] = [
        (
            "lettres/relance-fournisseur.txt",
            "Madame, Monsieur,\nNous vous relançons au sujet de la palissade commandée le 3 mars : \
             la livraison accuse trois semaines de retard.\nMerci de nous confirmer une date ferme.\n"
                .as_bytes()
                .to_vec(),
        ),
        ("photos/plan-masse.bin", photo),
        ("inventario/almacen.md", "# Inventario del almacén\n\nCemento: 40 sacos\nAndamios: 12 módulos\n".as_bytes().to_vec()),
    ];
    // 2024-03-15 10:30:00 UTC for every entry (stable fixtures).
    let date = NtTime::try_from(UNIX_EPOCH + Duration::from_secs(1_710_498_600)).expect("valid date");

    let mut writer = ArchiveWriter::create(&out).expect("create 7z");
    let mut archive_entries = Vec::new();
    let mut readers = Vec::new();
    for (name, bytes) in entries {
        let mut entry = ArchiveEntry::new_file(name);
        entry.last_modified_date = date;
        entry.has_last_modified_date = true;
        archive_entries.push(entry);
        readers.push(SourceReader::new(std::io::Cursor::new(bytes)));
    }
    writer.push_archive_entries(archive_entries, readers).expect("solid block");
    writer.finish().expect("finish 7z");
    println!("written {out}");
}
