//! The models that compute the meaning index in the background (lot 8.2).
//! Normal pace: 2 models, each on half of the cores minus one; economy: one
//! model on 2 cores. Every thread — ours and the runtime's — runs in the
//! background mode of Windows (lower CPU, disk and memory priority).

use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::{SenseModel, DIM};
use crate::error::Result;

/// How much of the PC the computation may use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Pace {
    #[default]
    Normal,
    Economy,
}

/// Lowers the priority of the calling thread (background mode).
pub fn background_thread() {
    #[cfg(windows)]
    {
        use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN};
        // SAFETY: the pseudo-handle of the calling thread.
        unsafe {
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN);
        }
    }
}

/// Creates ONNX Runtime's threads in background mode.
pub struct BackgroundThreads;

impl ort::environment::ThreadManager for BackgroundThreads {
    type Thread = std::thread::JoinHandle<()>;

    fn create(&self, work: impl FnOnce() + Send + 'static) -> ort::Result<Self::Thread> {
        Ok(std::thread::spawn(move || {
            background_thread();
            work();
        }))
    }

    fn join(thread: Self::Thread) -> ort::Result<()> {
        let _ = thread.join();
        Ok(())
    }
}

pub struct SensePool {
    models: Vec<SenseModel>,
    pub pace: Pace,
}

impl SensePool {
    pub fn load(dir: &Path, pace: Pace) -> Result<Self> {
        let cores = std::thread::available_parallelism().map_or(2, |n| n.get());
        let (count, threads) = match pace {
            Pace::Normal => (2, (cores.saturating_sub(1) / 2).max(1)),
            Pace::Economy => (1, 2.min(cores)),
        };
        let models = (0..count).map(|_| SenseModel::load_background(dir, threads)).collect::<Result<Vec<_>>>()?;
        Ok(Self { models, pace })
    }

    /// One vector per text, the texts shared between the models.
    pub fn embed(&self, texts: &[String]) -> Result<Vec<[f32; DIM]>> {
        let n = self.models.len();
        if n <= 1 || texts.len() < 2 {
            let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
            background_thread();
            return self.models[0].embed(&refs);
        }
        let results: Vec<Result<Vec<[f32; DIM]>>> = std::thread::scope(|scope| {
            let handles: Vec<_> = self
                .models
                .iter()
                .enumerate()
                .map(|(k, model)| {
                    let mine: Vec<&str> = texts.iter().skip(k).step_by(n).map(String::as_str).collect();
                    scope.spawn(move || {
                        background_thread();
                        model.embed(&mine)
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap_or_else(|_| Ok(Vec::new()))).collect()
        });
        let mut parts = results.into_iter().collect::<Result<Vec<_>>>()?;
        // Interleaved back in the order of the texts.
        let mut out = Vec::with_capacity(texts.len());
        let mut iters: Vec<_> = parts.iter_mut().map(|p| p.drain(..)).collect();
        for i in 0..texts.len() {
            if let Some(v) = iters[i % n].next() {
                out.push(v);
            }
        }
        Ok(out)
    }

    /// Passages of about 120 words per second on this PC (a few seconds of
    /// measure: the estimate shown before computing a site).
    pub fn speed(&self) -> Result<f32> {
        let sentence = "Le présent contrat prend effet à la date de signature et reste valable trois ans, sauf résiliation. ";
        let texts: Vec<String> = (0..8 * self.models.len()).map(|i| format!("{}{}", sentence.repeat(7), i)).collect();
        self.embed(&texts[..self.models.len()])?; // warm-up
        let started = Instant::now();
        self.embed(&texts)?;
        Ok(texts.len() as f32 / started.elapsed().as_secs_f32().max(0.001))
    }
}
