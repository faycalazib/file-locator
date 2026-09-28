//! Meaning search (Étape 8): an optional module, downloaded once, turns a
//! text into a vector of 384 numbers whose direction carries its meaning, in
//! any of its 52 languages ("contrat de location", "lease agreement" and
//! "عقد إيجار" point the same way). Nothing leaves the PC.
//!
//! - Model: Granite Embedding 97M multilingual r2 (IBM, Apache 2.0),
//!   quantized ONNX; the vector is the CLS token's, normalized.
//! - Runtime: ONNX Runtime, loaded at run time from the module folder
//!   (`ort`, `load-dynamic`): without the module, nothing is loaded.
//! - Tokenizer: the model's `tokenizer.json` (`tokenizers`).

pub mod module;

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Tensor;
use tokenizers::{Tokenizer, TruncationParams};

use crate::error::{CoreError, Result};

/// Numbers per vector.
pub const DIM: usize = 384;
/// Tokens read per text (about 250 words): longer texts are cut into passages.
pub const MAX_TOKENS: usize = 512;

fn fail(what: &str, e: impl std::fmt::Display) -> CoreError {
    CoreError::Sense { message: format!("{what}: {e}") }
}

/// ONNX Runtime is loaded once per process, from the first module folder.
static RUNTIME: OnceLock<std::result::Result<(), String>> = OnceLock::new();

fn load_runtime(dir: &Path) -> Result<()> {
    RUNTIME
        .get_or_init(|| {
            let dll = dir.join(if cfg!(windows) { "onnxruntime.dll" } else { "libonnxruntime.so" });
            let builder = ort::init_from(&dll).map_err(|e| format!("{}: {e}", dll.display()))?;
            builder.with_name("prospector").commit();
            Ok(())
        })
        .clone()
        .map_err(|e| fail("ONNX Runtime", e))
}

/// The loaded model.
pub struct SenseModel {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    /// The model's input names (`input_ids`, `attention_mask`, maybe `token_type_ids`).
    inputs: Vec<String>,
}

impl SenseModel {
    /// Loads the module installed in `dir` (see [`module::status`]).
    pub fn load(dir: &Path) -> Result<Self> {
        // Leave a core to the interface and the indexing.
        let threads = std::thread::available_parallelism().map_or(2, |n| n.get().saturating_sub(1).max(1));
        Self::load_with(dir, "model.onnx", threads)
    }

    /// Same, with a given model file and number of threads (measures).
    pub fn load_with(dir: &Path, model_file: &str, threads: usize) -> Result<Self> {
        load_runtime(dir)?;
        let session = Session::builder()
            .map_err(|e| fail("session", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| fail("session", e))?
            .with_intra_threads(threads)
            .map_err(|e| fail("session", e))?
            .commit_from_file(dir.join(model_file))
            .map_err(|e| fail("model", e))?;
        let inputs = session.inputs().iter().map(|i| i.name().to_owned()).collect();
        let mut tokenizer = Tokenizer::from_file(dir.join("tokenizer.json")).map_err(|e| fail("tokenizer", e))?;
        tokenizer
            .with_truncation(Some(TruncationParams { max_length: MAX_TOKENS, ..Default::default() }))
            .map_err(|e| fail("tokenizer", e))?;
        // No padding: a padded text does not get exactly the same vector as
        // alone (quantized model); texts are batched by equal length instead.
        tokenizer.with_padding(None);
        Ok(Self { session: Mutex::new(session), tokenizer, inputs })
    }

    /// One normalized vector per text. Texts of the same length (in tokens)
    /// go through the model together; a text always gets the same vector,
    /// whatever it is batched with.
    pub fn embed(&self, texts: &[&str]) -> Result<Vec<[f32; DIM]>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let encodings = self.tokenizer.encode_batch(texts.to_vec(), true).map_err(|e| fail("tokenizer", e))?;
        let mut by_length: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
        for (i, encoding) in encodings.iter().enumerate() {
            by_length.entry(encoding.get_ids().len().max(1)).or_default().push(i);
        }
        let mut out = vec![[0f32; DIM]; texts.len()];
        for (len, members) in by_length {
            let group: Vec<&tokenizers::Encoding> = members.iter().map(|&i| &encodings[i]).collect();
            for (slot, vector) in members.iter().zip(self.run(&group, len)?) {
                out[*slot] = vector;
            }
        }
        Ok(out)
    }

    /// One model run on texts of exactly `len` tokens.
    fn run(&self, group: &[&tokenizers::Encoding], len: usize) -> Result<Vec<[f32; DIM]>> {
        let batch = group.len();
        let column = |pick: &dyn Fn(&tokenizers::Encoding) -> &[u32]| -> Vec<i64> {
            let mut out = vec![0i64; batch * len];
            for (row, encoding) in group.iter().enumerate() {
                for (col, &v) in pick(encoding).iter().enumerate().take(len) {
                    out[row * len + col] = i64::from(v);
                }
            }
            out
        };
        let shape = [batch as i64, len as i64];
        let mut values: Vec<(String, Tensor<i64>)> = Vec::new();
        for name in &self.inputs {
            let data = match name.as_str() {
                "input_ids" => column(&|e| e.get_ids()),
                "attention_mask" => column(&|e| e.get_attention_mask()),
                "token_type_ids" => column(&|e| e.get_type_ids()),
                other => return Err(fail("model", format!("unexpected input {other}"))),
            };
            values.push((name.clone(), Tensor::from_array((shape, data)).map_err(|e| fail("tensor", e))?));
        }
        let mut session = self.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let outputs = session.run(values).map_err(|e| fail("model", e))?;
        let (out_shape, data) = outputs[0].try_extract_tensor::<f32>().map_err(|e| fail("model", e))?;
        // [batch, tokens, 384]: the first token (CLS) of each text.
        let dims: Vec<usize> = out_shape.iter().map(|&d| d as usize).collect();
        let stride = match dims.as_slice() {
            [b, t, d] if *d == DIM && *b == batch => t * d,
            [b, d] if *d == DIM && *b == batch => *d,
            _ => return Err(fail("model", format!("unexpected output shape {dims:?}"))),
        };
        Ok((0..batch)
            .map(|row| {
                let mut v = [0f32; DIM];
                v.copy_from_slice(&data[row * stride..row * stride + DIM]);
                normalize(&mut v);
                v
            })
            .collect())
    }
}

fn normalize(v: &mut [f32; DIM]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// Cosine similarity of two normalized vectors (1 = same meaning).
pub fn similarity(a: &[f32; DIM], b: &[f32; DIM]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
