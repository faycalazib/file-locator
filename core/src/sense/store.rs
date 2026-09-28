//! The meaning index of a site (lot 8.2): one Tantivy index next to the
//! site's (`indexes/<id>.sense`), one document per passage. Its vector is
//! quantized on 8 bits (a scale + 384 signed bytes) and packed in 48 `u64`
//! fast columns, which Tantivy reads very fast in blocks; everything else is
//! Tantivy's (deletion by file, merges, atomic commits, readers of a shared
//! index). The search goes through every passage (exact, no approximation).

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::path::Path;

use tantivy::schema::{Field, NumericOptions, Schema, Value, FAST, STORED, STRING};
use tantivy::{doc, DocAddress, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term};

use super::DIM;
use crate::error::{CoreError, Result};

/// `u64` columns holding one vector (8 bytes each).
const COLUMNS: usize = DIM / 8;
/// Documents scored together (one read of each column per block).
const BLOCK: usize = 512;

#[derive(Clone, Copy)]
struct Fields {
    path: Field,
    file: Field,
    passage: Field,
    start: Field,
    end: Field,
    scale: Field,
    v: [Field; COLUMNS],
}

fn schema() -> (Schema, Fields) {
    let mut b = Schema::builder();
    let number = NumericOptions::default().set_stored();
    let path = b.add_text_field("path", STRING | STORED);
    let file = b.add_text_field("file", STRING | STORED);
    let passage = b.add_u64_field("passage", number.clone());
    let start = b.add_u64_field("start", number.clone());
    let end = b.add_u64_field("end", number);
    let scale = b.add_f64_field("scale", FAST);
    let v = std::array::from_fn(|i| b.add_u64_field(&format!("v{i}"), FAST));
    (b.build(), Fields { path, file, passage, start, end, scale, v })
}

/// A passage found by its meaning.
#[derive(Clone, Debug, PartialEq)]
pub struct PassageHit {
    /// The document (`file`, or `file › inner`).
    pub path: String,
    pub passage: usize,
    /// Its place in the document's text (bytes).
    pub start: usize,
    pub end: usize,
    /// Cosine similarity with the query (1 = same meaning).
    pub score: f32,
}

/// A vector on 8 bits: its scale and the 48 packed words.
fn quantize(v: &[f32; DIM]) -> (f64, [u64; COLUMNS]) {
    let max = v.iter().fold(0f32, |m, x| m.max(x.abs()));
    let scale = if max > 0.0 { max / 127.0 } else { 1.0 };
    let mut words = [0u64; COLUMNS];
    for (c, word) in words.iter_mut().enumerate() {
        let mut bytes = [0u8; 8];
        for (k, byte) in bytes.iter_mut().enumerate() {
            *byte = ((v[c * 8 + k] / scale).round().clamp(-127.0, 127.0) as i8) as u8;
        }
        *word = u64::from_le_bytes(bytes);
    }
    (f64::from(scale), words)
}

pub struct SenseIndex {
    index: Index,
    pub reader: IndexReader,
    f: Fields,
}

fn index_error(e: impl std::fmt::Display) -> CoreError {
    CoreError::Index { message: e.to_string() }
}

impl SenseIndex {
    /// Opens (or creates) the meaning index in `dir`; `existing`: another
    /// PC writes it (shared index), nothing is created.
    pub fn open(dir: &Path, existing: bool) -> Result<Self> {
        let (schema, f) = schema();
        let index = if existing {
            let index = Index::open_in_dir(dir)?;
            if index.schema() != schema {
                return Err(index_error("meaning index written by another version"));
            }
            index
        } else {
            std::fs::create_dir_all(dir)?;
            match Index::open_or_create(tantivy::directory::MmapDirectory::open(dir)?, schema.clone()) {
                Ok(index) => index,
                // Another format: started again (it is computed from the main index).
                Err(tantivy::TantivyError::SchemaError(_)) => {
                    std::fs::remove_dir_all(dir)?;
                    std::fs::create_dir_all(dir)?;
                    Index::create_in_dir(dir, schema)?
                }
                Err(e) => return Err(e.into()),
            }
        };
        let reader = index.reader_builder().reload_policy(ReloadPolicy::Manual).try_into()?;
        Ok(Self { index, reader, f })
    }

    pub fn writer(&self) -> Result<IndexWriter> {
        Ok(self.index.writer(32 * 1024 * 1024)?)
    }

    /// Adds one passage.
    pub fn add(&self, writer: &IndexWriter, path: &str, file: &str, passage: usize, (start, end): (usize, usize), v: &[f32; DIM]) -> Result<()> {
        let (scale, words) = quantize(v);
        let f = self.f;
        let mut document = doc!(
            f.path => path,
            f.file => file,
            f.passage => passage as u64,
            f.start => start as u64,
            f.end => end as u64,
            f.scale => scale,
        );
        for (field, word) in f.v.iter().zip(words) {
            document.add_u64(*field, word);
        }
        writer.add_document(document)?;
        Ok(())
    }

    /// Removes every passage of a file on disk (all its documents).
    pub fn delete_file(&self, writer: &IndexWriter, file: &str) {
        writer.delete_term(Term::from_field_text(self.f.file, file));
    }

    /// Passages in the index.
    pub fn len(&self) -> u64 {
        self.reader.searcher().num_docs()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `limit` passages closest to `query` (normalized), best first.
    pub fn search(&self, query: &[f32; DIM], limit: usize) -> Result<Vec<PassageHit>> {
        #[derive(PartialEq)]
        struct Scored(f32, u32, u32);
        impl Eq for Scored {}
        impl PartialOrd for Scored {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        // Reversed: the heap keeps the best `limit`, its top is the worst of them.
        impl Ord for Scored {
            fn cmp(&self, other: &Self) -> Ordering {
                other.0.total_cmp(&self.0)
            }
        }
        if limit == 0 {
            return Ok(Vec::new());
        }
        let searcher = self.reader.searcher();
        let mut heap: BinaryHeap<Scored> = BinaryHeap::with_capacity(limit + 1);
        let mut words = vec![0u64; BLOCK];
        let mut scales = vec![0f64; BLOCK];
        let mut scores = vec![0f32; BLOCK];
        for (ordinal, segment) in searcher.segment_readers().iter().enumerate() {
            let fast = segment.fast_fields();
            let columns = (0..COLUMNS).map(|i| fast.u64(&format!("v{i}"))).collect::<tantivy::Result<Vec<_>>>()?;
            let scale = fast.f64("scale")?;
            let alive = segment.alive_bitset();
            let max = segment.max_doc() as usize;
            let mut first = 0usize;
            while first < max {
                let n = BLOCK.min(max - first);
                scores[..n].fill(0.0);
                for (c, column) in columns.iter().enumerate() {
                    column.values.get_range(first as u64, &mut words[..n]);
                    let q = &query[c * 8..c * 8 + 8];
                    for (score, word) in scores[..n].iter_mut().zip(&words[..n]) {
                        let b = word.to_le_bytes();
                        *score += f32::from(b[0] as i8) * q[0]
                            + f32::from(b[1] as i8) * q[1]
                            + f32::from(b[2] as i8) * q[2]
                            + f32::from(b[3] as i8) * q[3]
                            + f32::from(b[4] as i8) * q[4]
                            + f32::from(b[5] as i8) * q[5]
                            + f32::from(b[6] as i8) * q[6]
                            + f32::from(b[7] as i8) * q[7];
                    }
                }
                scale.values.get_range(first as u64, &mut scales[..n]);
                for d in 0..n {
                    let doc = (first + d) as u32;
                    if alive.is_some_and(|a| !a.is_alive(doc)) {
                        continue;
                    }
                    let score = scores[d] * scales[d] as f32;
                    if heap.len() < limit {
                        heap.push(Scored(score, ordinal as u32, doc));
                    } else if heap.peek().is_some_and(|worst| score > worst.0) {
                        heap.pop();
                        heap.push(Scored(score, ordinal as u32, doc));
                    }
                }
                first += n;
            }
        }
        let mut best: Vec<Scored> = heap.into_vec();
        best.sort_by(|a, b| b.0.total_cmp(&a.0));
        best.into_iter()
            .map(|Scored(score, ordinal, doc)| {
                let stored: TantivyDocument = searcher.doc(DocAddress::new(ordinal, doc))?;
                let text = |f: Field| stored.get_first(f).and_then(|v| v.as_str()).unwrap_or_default().to_owned();
                let number = |f: Field| stored.get_first(f).and_then(|v| v.as_u64()).unwrap_or_default() as usize;
                Ok(PassageHit { path: text(self.f.path), passage: number(self.f.passage), start: number(self.f.start), end: number(self.f.end), score })
            })
            .collect()
    }

    /// Rewrites the paths (portable drive with another letter, lot 6.8):
    /// every passage whose path `remap` changes is written again, its vector
    /// read back from the columns. Returns the number of passages moved.
    pub fn remap_paths(&self, remap: &dyn Fn(&str) -> Option<String>) -> Result<usize> {
        let searcher = self.reader.searcher();
        let mut writer = self.writer()?;
        let mut moved = 0;
        for (ordinal, segment) in searcher.segment_readers().iter().enumerate() {
            let fast = segment.fast_fields();
            let columns = (0..COLUMNS).map(|i| fast.u64(&format!("v{i}"))).collect::<tantivy::Result<Vec<_>>>()?;
            let scale = fast.f64("scale")?;
            for doc in segment.doc_ids_alive() {
                let stored: TantivyDocument = searcher.doc(DocAddress::new(ordinal as u32, doc))?;
                let text = |f: Field| stored.get_first(f).and_then(|v| v.as_str()).unwrap_or_default().to_owned();
                let number = |f: Field| stored.get_first(f).and_then(|v| v.as_u64()).unwrap_or_default();
                let old = text(self.f.path);
                let Some(new_path) = remap(&old) else { continue };
                let new_file = remap(&text(self.f.file)).unwrap_or_else(|| text(self.f.file));
                let f = self.f;
                let mut document = doc!(
                    f.path => new_path,
                    f.file => new_file,
                    f.passage => number(f.passage),
                    f.start => number(f.start),
                    f.end => number(f.end),
                    f.scale => scale.first(doc).unwrap_or(1.0),
                );
                for (field, column) in f.v.iter().zip(&columns) {
                    document.add_u64(*field, column.first(doc).unwrap_or_default());
                }
                writer.delete_term(Term::from_field_text(f.path, &old));
                writer.add_document(document)?;
                moved += 1;
            }
        }
        if moved > 0 {
            writer.commit()?;
        }
        writer.wait_merging_threads()?;
        Ok(moved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(seed: u32) -> [f32; DIM] {
        // A deterministic, spread vector.
        let mut v = [0f32; DIM];
        let mut x = seed.wrapping_mul(2_654_435_761) | 1;
        for value in v.iter_mut() {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            *value = (x % 2001) as f32 / 1000.0 - 1.0;
        }
        let norm = v.iter().map(|a| a * a).sum::<f32>().sqrt();
        v.iter_mut().for_each(|a| *a /= norm);
        v
    }

    #[test]
    fn nearest_passages_deletion_and_remap() {
        let dir = crate::test_tmp();
        let index = SenseIndex::open(dir.path(), false).unwrap();
        let mut writer = index.writer().unwrap();
        // 1,500 passages: more than two blocks.
        for i in 0..1500u32 {
            let file = format!(r"E:\docs\f{}.txt", i / 10);
            index.add(&writer, &file, &file, (i % 10) as usize, (0, 10), &unit(i)).unwrap();
        }
        writer.commit().unwrap();
        index.reader.reload().unwrap();
        assert_eq!(index.len(), 1500);

        // The query is passage 777 itself: found first, with a score near 1
        // (8 bits lose a little).
        let hits = index.search(&unit(777), 5).unwrap();
        assert_eq!((hits[0].path.as_str(), hits[0].passage), (r"E:\docs\f77.txt", 7));
        assert!(hits[0].score > 0.99, "{}", hits[0].score);
        assert!(hits.windows(2).all(|w| w[0].score >= w[1].score));

        // A deleted file is never found again.
        index.delete_file(&writer, r"E:\docs\f77.txt");
        writer.commit().unwrap();
        index.reader.reload().unwrap();
        assert!(index.search(&unit(777), 5).unwrap().iter().all(|h| h.path != r"E:\docs\f77.txt"));
        drop(writer);

        // Another drive letter: paths move, vectors stay.
        let moved = index.remap_paths(&|p| p.strip_prefix("E:").map(|rest| format!("F:{rest}"))).unwrap();
        assert_eq!(moved, 1490);
        index.reader.reload().unwrap();
        let hit = &index.search(&unit(781), 1).unwrap()[0];
        assert_eq!((hit.path.as_str(), hit.passage), (r"F:\docs\f78.txt", 1));
        assert!(hit.score > 0.99);
    }
}
