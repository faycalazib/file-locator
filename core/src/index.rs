//! Tantivy index of one dig site (goal.md §2 `index/`).
//!
//! Schema:
//! - `path` (raw, stored)         — identity of a document
//! - `file` (raw)                 — the file on disk it comes from (itself, or
//!   the ZIP / PST holding it): deleting this term removes all its documents
//! - `name` (generic analyzer)    — file name, searchable and boosted
//! - `name_raw` (raw)             — the name folded (no case, no accents), for
//!   the file-name criterion (`*.pdf`, see names.rs)
//! - `kind`, `lang` (raw, stored, fast) — filters, counts per criterion (lot 6.3)
//! - `size`, `modified`, `created` (u64) — filters, displayed (`created`:
//!   lot 5.8; an index without it is migrated from its stored text)
//! - `body` (generic analyzer, positions, stored) — accent/vowel-insensitive
//!   search, phrases, fuzzy; stored for snippets and preview
//! - `stem_en|fr|es|ar`           — the text again, stemmed in its language

use std::path::Path;

use tantivy::directory::MmapDirectory;
use tantivy::schema::{
    Field, IndexRecordOption, NumericOptions, Schema, TextFieldIndexing, TextOptions, FAST, STORED, STRING,
};
use tantivy::{doc, Index, IndexReader, ReloadPolicy, TantivyDocument, TantivyError};

use crate::disk::Facts;
use crate::error::Result;
use crate::extract::split_inner;
use crate::lang::{self, DocLang, GENERIC};

#[derive(Clone, Copy, Debug)]
pub struct Fields {
    pub path: Field,
    pub file: Field,
    pub name: Field,
    pub name_raw: Field,
    pub kind: Field,
    pub lang: Field,
    pub size: Field,
    pub modified: Field,
    pub created: Field,
    pub body: Field,
    pub stem_en: Field,
    pub stem_fr: Field,
    pub stem_es: Field,
    pub stem_ar: Field,
}

impl Fields {
    pub fn stem(&self, lang: DocLang) -> Option<Field> {
        match lang {
            DocLang::En => Some(self.stem_en),
            DocLang::Fr => Some(self.stem_fr),
            DocLang::Es => Some(self.stem_es),
            DocLang::Ar => Some(self.stem_ar),
            DocLang::Und => None,
        }
    }
}

fn text(analyzer: &str, record: IndexRecordOption, stored: bool) -> TextOptions {
    let options = TextOptions::default()
        .set_indexing_options(TextFieldIndexing::default().set_tokenizer(analyzer).set_index_option(record));
    if stored {
        options.set_stored()
    } else {
        options
    }
}

pub fn build_schema() -> (Schema, Fields) {
    let mut b = Schema::builder();
    let number = NumericOptions::default().set_indexed().set_stored().set_fast();
    let stem = |lang: DocLang| text(&lang.analyzer_name(), IndexRecordOption::WithFreqs, false);
    let fields = Fields {
        path: b.add_text_field("path", STRING | STORED),
        file: b.add_text_field("file", STRING),
        name: b.add_text_field("name", text(GENERIC, IndexRecordOption::WithFreqsAndPositions, true)),
        name_raw: b.add_text_field("name_raw", STRING),
        // Fast: counted per criterion (lot 6.3).
        kind: b.add_text_field("kind", STRING | STORED | FAST),
        lang: b.add_text_field("lang", STRING | STORED | FAST),
        size: b.add_u64_field("size", number.clone()),
        modified: b.add_u64_field("modified", number.clone()),
        created: b.add_u64_field("created", number),
        body: b.add_text_field("body", text(GENERIC, IndexRecordOption::WithFreqsAndPositions, true)),
        stem_en: b.add_text_field("stem_en", stem(DocLang::En)),
        stem_fr: b.add_text_field("stem_fr", stem(DocLang::Fr)),
        stem_es: b.add_text_field("stem_es", stem(DocLang::Es)),
        stem_ar: b.add_text_field("stem_ar", stem(DocLang::Ar)),
    };
    (b.build(), fields)
}

/// An open index with its reader.
pub struct SiteIndex {
    pub index: Index,
    pub reader: IndexReader,
    pub fields: Fields,
}

impl SiteIndex {
    pub fn open_or_create(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let (schema, fields) = build_schema();
        let index = match Index::open_or_create(MmapDirectory::open(dir)?, schema.clone()) {
            Ok(index) => index,
            // Written by an older version (the schema changed): rewritten
            // from its stored text when it has it (no file read again, no
            // OCR again); otherwise started again empty, and the next
            // update of the site rebuilds it from the files.
            Err(TantivyError::SchemaError(_)) => match migrate(dir, &schema, &fields) {
                Ok(index) => index,
                Err(_) => {
                    let _ = std::fs::remove_dir_all(dir.with_extension("migrating"));
                    std::fs::remove_dir_all(dir)?;
                    std::fs::create_dir_all(dir)?;
                    Index::create_in_dir(dir, schema)?
                }
            },
            Err(e) => return Err(e.into()),
        };
        lang::register(index.tokenizers());
        let reader = index.reader_builder().reload_policy(ReloadPolicy::Manual).try_into()?;
        Ok(Self { index, reader, fields })
    }
}

impl SiteIndex {
    /// Opens an index another PC writes (shared index, lot 7.3): nothing is
    /// created or migrated; one written by another version is refused.
    pub fn open_existing(dir: &Path) -> Result<Self> {
        let (schema, fields) = build_schema();
        let index = Index::open_in_dir(dir)?;
        if index.schema() != schema {
            return Err(crate::error::CoreError::Index { message: "index written by another version of Prospector".to_owned() });
        }
        lang::register(index.tokenizers());
        let reader = index.reader_builder().reload_policy(ReloadPolicy::Manual).try_into()?;
        Ok(Self { index, reader, fields })
    }
}

/// Everything a Tantivy document is made of.
pub struct DocParts<'a> {
    /// Identity: the file, or `file › inner` for a document inside it.
    pub path: &'a str,
    /// The file on disk.
    pub file: &'a str,
    pub name: &'a str,
    /// Inside an archive or a mailbox: the document also answers to the name
    /// of its container (`*.rar` finds the documents of every RAR).
    pub inner: bool,
    pub kind: &'a str,
    pub lang: DocLang,
    pub text: &'a str,
    pub size: u64,
    pub modified: u64,
    pub created: u64,
}

pub fn document(f: &Fields, parts: &DocParts) -> TantivyDocument {
    let mut document = doc!(
        f.path => parts.path,
        f.file => parts.file,
        f.name_raw => lang::fold(parts.name),
        f.name => parts.name,
        f.kind => parts.kind,
        f.lang => parts.lang.code(),
        f.size => parts.size,
        f.modified => parts.modified,
        f.created => parts.created,
    );
    if parts.inner {
        if let Some(container) = Path::new(parts.file).file_name() {
            document.add_text(f.name_raw, lang::fold(&container.to_string_lossy()));
        }
    }
    if let Some(stem_field) = f.stem(parts.lang) {
        document.add_text(stem_field, parts.text);
    }
    document.add_text(f.body, parts.text);
    document
}

/// Rewrites an index made with an older schema from what it stores (path,
/// name, kind, language, size, date, text): the other fields are derived,
/// the creation date is read on the disk (the modification date if the file
/// is gone). The new index is written aside, then takes the old one's place.
fn migrate(dir: &Path, schema: &Schema, fields: &Fields) -> Result<Index> {
    use tantivy::schema::Value;
    use tantivy::DocAddress;

    let old = Index::open_in_dir(dir)?;
    let old_schema = old.schema();
    let field = |name: &str| old_schema.get_field(name);
    let (path, name, kind, lang_field, size, modified, body) = (
        field("path")?,
        field("name")?,
        field("kind")?,
        field("lang")?,
        field("size")?,
        field("modified")?,
        field("body")?,
    );
    let reader = old.reader()?;
    let searcher = reader.searcher();

    let aside = dir.with_extension("migrating");
    if aside.exists() {
        std::fs::remove_dir_all(&aside)?;
    }
    std::fs::create_dir_all(&aside)?;
    let new = Index::create_in_dir(&aside, schema.clone())?;
    lang::register(new.tokenizers());
    let mut writer: tantivy::IndexWriter = new.writer(128 * 1024 * 1024)?;
    for (ordinal, segment) in searcher.segment_readers().iter().enumerate() {
        for doc_id in segment.doc_ids_alive() {
            let doc: TantivyDocument = searcher.doc(DocAddress::new(ordinal as u32, doc_id))?;
            let text = |f: Field| doc.get_first(f).and_then(|v| v.as_str()).unwrap_or_default().to_owned();
            let number = |f: Field| doc.get_first(f).and_then(|v| v.as_u64()).unwrap_or_default();
            let doc_path = text(path);
            let (file, inner) = split_inner(&doc_path);
            let doc_modified = number(modified);
            let created = Facts::read(Path::new(file)).map_or(doc_modified, |f| f.created);
            let parts = DocParts {
                path: &doc_path,
                file,
                name: &text(name),
                inner: inner.is_some(),
                kind: &text(kind),
                lang: DocLang::from_code(&text(lang_field)).unwrap_or(DocLang::Und),
                text: &text(body),
                size: number(size),
                modified: doc_modified,
                created,
            };
            writer.add_document(document(fields, &parts))?;
        }
    }
    writer.commit()?;
    writer.wait_merging_threads()?;
    drop(new);
    drop(searcher);
    drop(reader);
    drop(old);

    // Swap: the old index goes aside first, so a failure leaves one of them.
    let previous = dir.with_extension("previous");
    if previous.exists() {
        std::fs::remove_dir_all(&previous)?;
    }
    std::fs::rename(dir, &previous)?;
    std::fs::rename(&aside, dir)?;
    let _ = std::fs::remove_dir_all(&previous);
    Ok(Index::open_in_dir(dir)?)
}

/// Rewrites the paths of an index (portable mode, lot 6.8: the USB drive got
/// another letter): every document whose path `remap` changes is written
/// again from what the index stores, nothing is read on the disk. One
/// commit: an interruption leaves the index as it was. Returns the number
/// of documents moved.
pub fn remap_paths(dir: &Path, remap: &dyn Fn(&str) -> Option<String>) -> Result<usize> {
    use tantivy::schema::Value;
    use tantivy::DocAddress;

    let site = SiteIndex::open_or_create(dir)?;
    let f = site.fields;
    let searcher = site.reader.searcher();
    let mut writer: tantivy::IndexWriter = site.index.writer(64 * 1024 * 1024)?;
    let mut moved = 0;
    for (ordinal, segment) in searcher.segment_readers().iter().enumerate() {
        for doc_id in segment.doc_ids_alive() {
            let doc: TantivyDocument = searcher.doc(DocAddress::new(ordinal as u32, doc_id))?;
            let text = |field: Field| doc.get_first(field).and_then(|v| v.as_str()).unwrap_or_default().to_owned();
            let number = |field: Field| doc.get_first(field).and_then(|v| v.as_u64()).unwrap_or_default();
            let old_path = text(f.path);
            let Some(new_path) = remap(&old_path) else { continue };
            let (file, inner) = split_inner(&new_path);
            let parts = DocParts {
                path: &new_path,
                file,
                name: &text(f.name),
                inner: inner.is_some(),
                kind: &text(f.kind),
                lang: DocLang::from_code(&text(f.lang)).unwrap_or(DocLang::Und),
                text: &text(f.body),
                size: number(f.size),
                modified: number(f.modified),
                created: number(f.created),
            };
            writer.delete_term(tantivy::Term::from_field_text(f.path, &old_path));
            writer.add_document(document(&f, &parts))?;
            moved += 1;
        }
    }
    if moved > 0 {
        writer.commit()?;
    }
    writer.wait_merging_threads()?;
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::collector::{Count, TopDocs};
    use tantivy::query::TermQuery;
    use tantivy::schema::Value;
    use tantivy::Term;

    /// The schema of lot 5.7: no `created`.
    fn previous_schema() -> Schema {
        let mut b = Schema::builder();
        let number = NumericOptions::default().set_indexed().set_stored().set_fast();
        let stem = |lang: DocLang| text(&lang.analyzer_name(), IndexRecordOption::WithFreqs, false);
        b.add_text_field("path", STRING | STORED);
        b.add_text_field("file", STRING);
        b.add_text_field("name", text(GENERIC, IndexRecordOption::WithFreqsAndPositions, true));
        b.add_text_field("name_raw", STRING);
        b.add_text_field("kind", STRING | STORED);
        b.add_text_field("lang", STRING | STORED);
        b.add_u64_field("size", number.clone());
        b.add_u64_field("modified", number);
        b.add_text_field("body", text(GENERIC, IndexRecordOption::WithFreqsAndPositions, true));
        for lang in [DocLang::En, DocLang::Fr, DocLang::Es, DocLang::Ar] {
            b.add_text_field(&format!("stem_{}", lang.code()), stem(lang));
        }
        b.build()
    }

    fn created_of(site: &SiteIndex, query: &TermQuery) -> Option<u64> {
        let searcher = site.reader.searcher();
        let top = searcher.search(query, &TopDocs::with_limit(1).order_by_score()).unwrap();
        let doc: TantivyDocument = searcher.doc(top.first()?.1).unwrap();
        doc.get_first(site.fields.created).and_then(|v| v.as_u64())
    }

    #[test]
    fn an_older_index_is_rewritten_from_its_stored_text() {
        let root = crate::test_tmp();
        let file = root.path().join("contrat.txt");
        std::fs::write(&file, "Le contrat est résilié.").unwrap();
        let file = file.to_string_lossy().into_owned();
        let absent = root.path().join("absent.zip").to_string_lossy().into_owned();
        let inner = format!("{absent}{}notes.txt", crate::extract::INNER_SEP);
        let dir = root.path().join("index");
        std::fs::create_dir_all(&dir).unwrap();
        {
            let schema = previous_schema();
            let index = Index::create_in_dir(&dir, schema.clone()).unwrap();
            lang::register(index.tokenizers());
            let mut writer: tantivy::IndexWriter = index.writer(20_000_000).unwrap();
            let f = |n: &str| schema.get_field(n).unwrap();
            for (path, on_disk, name) in [(file.as_str(), file.as_str(), "contrat.txt"), (inner.as_str(), absent.as_str(), "notes.txt")] {
                writer
                    .add_document(doc!(
                        f("path") => path, f("file") => on_disk, f("name") => name, f("name_raw") => name,
                        f("kind") => "text", f("lang") => "fr", f("size") => 24u64, f("modified") => 1_000u64,
                        f("body") => "Le contrat est résilié.", f("stem_fr") => "Le contrat est résilié.",
                    ))
                    .unwrap();
            }
            writer.commit().unwrap();
        }

        let site = SiteIndex::open_or_create(&dir).unwrap();
        let f = site.fields;
        let searcher = site.reader.searcher();
        assert_eq!(searcher.search(&tantivy::query::AllQuery, &Count).unwrap(), 2, "every document kept");
        let word = TermQuery::new(Term::from_field_text(f.body, "resilie"), IndexRecordOption::Basic);
        assert_eq!(searcher.search(&word, &Count).unwrap(), 2, "the text is still searchable");
        // `file` derived again, creation read on the disk.
        let by_file = TermQuery::new(Term::from_field_text(f.file, &file), IndexRecordOption::Basic);
        assert!(created_of(&site, &by_file).unwrap() > 1_000);
        // An inner document answers to its container's name; its file is gone: modification date.
        let by_container = TermQuery::new(Term::from_field_text(f.name_raw, "absent.zip"), IndexRecordOption::Basic);
        assert_eq!(created_of(&site, &by_container), Some(1_000));
        assert!(!dir.with_extension("migrating").exists() && !dir.with_extension("previous").exists());
    }
}
