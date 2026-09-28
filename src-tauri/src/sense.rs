//! Meaning search (Étape 8), on the app side.
//! - Lot 8.1, the module: its status, downloading it from the release
//!   (progress `sense://progress`, cancel), installing it from a file,
//!   removing it. It lives in this PC's personal folder.
//! - Lot 8.2, the meaning index: a thread computes, in the background, what
//!   remains for the sites whose "Meaning" box is ticked (`sense://site`
//!   progress). It wakes up after an indexing, when a box is ticked and when
//!   the module is installed; it only runs on the PC that keeps the index.

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use prospector_core::sense::module::{self, ModuleStatus, MODULE_BYTES, MODULE_SHA256, MODULE_URL};
use prospector_core::sense::pool::{Pace, SensePool};
use prospector_core::{CoreError, SenseProgress};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::commands::AppError;
use crate::state::AppState;

const CANCEL_KEY: &str = "sense-download";

fn dir(state: &AppState) -> PathBuf {
    module::module_dir(&state.personal_dir)
}

fn sense_error(message: impl Into<String>) -> AppError {
    AppError::Core(CoreError::Sense { message: message.into() })
}

/// At startup: finishes a removal asked before, cleans an interrupted install.
pub fn cleanup(state: &AppState) {
    module::cleanup(&dir(state));
    let _ = std::fs::remove_file(download_path(state));
}

fn download_path(state: &AppState) -> PathBuf {
    state.personal_dir.join("modules").join("sense-download.zip")
}

#[tauri::command]
pub fn sense_status(state: State<'_, AppState>) -> ModuleStatus {
    module::status(&dir(&state))
}

/// Installs the module from a ZIP chosen by the user (offline PCs, and
/// before the release exists): checked file by file against its manifest.
#[tauri::command]
pub async fn install_sense_module(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<ModuleStatus, AppError> {
    let target = dir(&state);
    let status = tauri::async_runtime::spawn_blocking(move || module::install_zip(std::path::Path::new(&path), &target, None).map(|_| module::status(&target)))
        .await
        .map_err(|_| AppError::TaskFailed)?
        .map_err(AppError::Core)?;
    // Sites already ticked can be computed now.
    wake(&app);
    Ok(status)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    done: u64,
    total: u64,
}

/// Downloads the module from the release, checks its SHA-256, installs it.
#[tauri::command]
pub async fn download_sense_module(app: AppHandle, state: State<'_, AppState>) -> Result<ModuleStatus, AppError> {
    let url = MODULE_URL.ok_or_else(|| sense_error("no download address in this version"))?;
    let cancel = Arc::new(AtomicBool::new(false));
    state.set_cancel(CANCEL_KEY, cancel.clone());
    let zip = download_path(&state);
    let result = download(&app, url, &zip, &cancel).await;
    if let Ok(mut map) = state.cancels.lock() {
        map.remove(CANCEL_KEY);
    }
    if let Err(e) = result {
        let _ = std::fs::remove_file(&zip);
        return Err(e);
    }
    let target = dir(&state);
    let installed = tauri::async_runtime::spawn_blocking(move || {
        let done = module::install_zip(&zip, &target, Some(MODULE_SHA256)).map(|_| module::status(&target));
        let _ = std::fs::remove_file(&zip);
        done
    })
    .await
    .map_err(|_| AppError::TaskFailed)?;
    let status = installed.map_err(AppError::Core)?;
    wake(&app);
    Ok(status)
}

async fn download(app: &AppHandle, url: &str, zip: &std::path::Path, cancel: &AtomicBool) -> Result<(), AppError> {
    // The same TLS as the updater (rustls, ring): nothing more to ship.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let offline = |_| sense_error("the download failed; check the connection");
    let mut response = reqwest::Client::new().get(url).send().await.map_err(offline)?.error_for_status().map_err(offline)?;
    let total = response.content_length().unwrap_or(MODULE_BYTES);
    if let Some(parent) = zip.parent() {
        std::fs::create_dir_all(parent).map_err(|_| AppError::WriteFailed { path: parent.to_string_lossy().into_owned() })?;
    }
    let mut file = std::fs::File::create(zip).map_err(|_| AppError::WriteFailed { path: zip.to_string_lossy().into_owned() })?;
    let mut done = 0u64;
    let mut reported = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(offline)? {
        if cancel.load(Ordering::Relaxed) {
            return Err(sense_error("download cancelled"));
        }
        file.write_all(&chunk).map_err(|_| AppError::WriteFailed { path: zip.to_string_lossy().into_owned() })?;
        done += chunk.len() as u64;
        if done - reported >= 512 * 1024 || done == total {
            reported = done;
            let _ = app.emit("sense://progress", Progress { done, total });
        }
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_sense_download(state: State<'_, AppState>) {
    if let Some(flag) = state.cancel_flag(CANCEL_KEY) {
        flag.store(true, Ordering::Relaxed);
    }
}

/// Removes the module (at the next start if it is in use).
#[tauri::command]
pub fn remove_sense_module(app: AppHandle, state: State<'_, AppState>) -> Result<ModuleStatus, AppError> {
    let runtime = app.state::<SenseRuntime>();
    // Whatever runs stops; the models go.
    for id in lock(&state.sense_progress).keys() {
        if let Some(flag) = state.cancel_flag(&cancel_key(id)) {
            flag.store(true, Ordering::Relaxed);
        }
    }
    *lock(&runtime.pool) = None;
    *lock(&runtime.questions) = None;
    let target = dir(&state);
    module::remove(&target).map_err(AppError::Core)?;
    Ok(module::status(&target))
}

// ── Lot 8.2: the meaning index, computed in the background ────────────────

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn cancel_key(site_id: &str) -> String {
    format!("sense:{site_id}")
}

/// The models (loaded when first needed), their measured speed, what is
/// being computed, and the bell that wakes the thread up.
pub struct SenseRuntime {
    pool: Mutex<Option<Arc<SensePool>>>,
    /// Passages per second on this PC, at the current pace.
    speed: Mutex<Option<f32>>,
    wake: Mutex<Option<Sender<()>>>,
    /// The model answering the questions of searches (lot 8.3).
    questions: Mutex<Option<Arc<prospector_core::sense::SenseModel>>>,
}

impl SenseRuntime {
    pub fn new() -> (Self, Receiver<()>) {
        let (tx, rx) = channel();
        (Self { pool: Mutex::new(None), speed: Mutex::new(None), wake: Mutex::new(Some(tx)), questions: Mutex::new(None) }, rx)
    }
}

/// Something new to compute (an indexing ended, a box ticked, the module installed).
pub fn wake(app: &AppHandle) {
    if let Some(runtime) = app.try_state::<SenseRuntime>() {
        if let Some(tx) = lock(&runtime.wake).as_ref() {
            let _ = tx.send(());
        }
    }
}

fn pace(state: &AppState) -> Pace {
    match crate::settings::Settings::load(&state.settings_path).sense_pace.as_deref() {
        Some("economy") => Pace::Economy,
        _ => Pace::Normal,
    }
}

/// The vector of a search's question (lot 8.3): a model kept for the
/// questions (loaded once, about a second; then about 20 ms a question).
/// `None`: no module, no text, or the model failed (the search is by words).
pub fn question_vector(app: &AppHandle, query: &str) -> Option<[f32; prospector_core::sense::DIM]> {
    let text = prospector_core::sense::question_text(query);
    if text.is_empty() {
        return None;
    }
    let state = app.state::<AppState>();
    let runtime = app.state::<SenseRuntime>();
    let model = {
        let mut slot = lock(&runtime.questions);
        if slot.is_none() {
            module::installed(&dir(&state))?;
            *slot = prospector_core::sense::SenseModel::load(&dir(&state)).ok().map(Arc::new);
        }
        slot.clone()?
    };
    model.embed(&[text.as_str()]).ok()?.into_iter().next()
}

/// The models, loaded once (a few seconds) with the chosen pace.
fn pool(app: &AppHandle) -> Result<Arc<SensePool>, CoreError> {
    let state = app.state::<AppState>();
    let runtime = app.state::<SenseRuntime>();
    let wanted = pace(&state);
    let mut slot = lock(&runtime.pool);
    if let Some(pool) = slot.as_ref().filter(|p| p.pace == wanted) {
        return Ok(pool.clone());
    }
    let pool = Arc::new(SensePool::load(&dir(&state), wanted)?);
    *lock(&runtime.speed) = pool.speed().ok();
    *slot = Some(pool.clone());
    Ok(pool)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SiteProgress {
    site_id: String,
    done: usize,
    total: usize,
    /// Finished (everything computed) or stopped.
    finished: bool,
}

/// The thread: waits for the bell (or 5 minutes), then computes what
/// remains, site by site, while this PC keeps the index and has the module.
pub fn start(app: &AppHandle, wake_rx: Receiver<()>) {
    let app = app.clone();
    std::thread::spawn(move || loop {
        match wake_rx.recv_timeout(Duration::from_secs(300)) {
            Ok(()) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        // Several bells at once: one round.
        while wake_rx.try_recv().is_ok() {}
        run_round(&app);
    });
}

fn run_round(app: &AppHandle) {
    let state = app.state::<AppState>();
    if module::installed(&dir(&state)).is_none() {
        return;
    }
    loop {
        let engine = state.engine();
        if engine.reader_of().is_some() {
            return;
        }
        let next = engine.sites().into_iter().find(|s| s.sense && s.last_indexed.is_some() && !engine.is_busy(&s.id) && engine.sense_pending(&s.id));
        let Some(site) = next else { return };
        let Ok(pool) = pool(app) else { return };
        let cancel = Arc::new(AtomicBool::new(false));
        state.set_cancel(&cancel_key(&site.id), cancel.clone());
        let emit = |p: SenseProgress, finished: bool| {
            lock(&state.sense_progress).insert(site.id.clone(), p);
            let _ = app.emit("sense://site", SiteProgress { site_id: site.id.clone(), done: p.done, total: p.total, finished });
        };
        let last = Mutex::new(std::time::Instant::now());
        let result = engine.sense_update(&site.id, &|texts: &[String]| pool.embed(texts), &cancel, &|p| {
            let mut last = lock(&last);
            if last.elapsed() >= Duration::from_millis(700) || p.done == p.total {
                *last = std::time::Instant::now();
                emit(p, false);
            }
        });
        if let Ok(mut map) = state.cancels.lock() {
            map.remove(&cancel_key(&site.id));
        }
        let finished = matches!(result, Ok(true));
        let p = lock(&state.sense_progress).remove(&site.id).unwrap_or(SenseProgress { done: 0, total: 0 });
        let _ = app.emit("sense://site", SiteProgress { site_id: site.id.clone(), done: p.done, total: p.total, finished });
        if !finished {
            // Stopped (box unticked, module removed, taken over) or failed: next bell.
            return;
        }
    }
}

/// The whole computation against the models alone (measured, lot 8.2).
const CHAIN_OVERHEAD: f32 = 1.25;

/// What ticking "Meaning" on a site will cost: passages and seconds at
/// this PC's speed (the models are loaded, and measured, if needed).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SenseEstimate {
    passages: usize,
    seconds: Option<u64>,
    /// Passages already computed.
    done: u64,
}

#[tauri::command]
pub async fn sense_estimate(app: AppHandle, id: String) -> Result<SenseEstimate, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let plan = state.engine().sense_plan(&id, true).map_err(AppError::Core)?;
        let speed = if module::installed(&dir(&state)).is_some() { pool(&app).ok().and(*lock(&app.state::<SenseRuntime>().speed)) } else { None };
        // The whole chain (reading the index, commits) is about 25 % slower than
        // the models alone: measured on 100,000 files, 52 min for 41 estimated.
        let seconds = speed.filter(|s| *s > 0.0).map(|s| (plan.passages as f32 * CHAIN_OVERHEAD / s).ceil() as u64);
        Ok(SenseEstimate { passages: plan.passages, seconds, done: plan.done })
    })
    .await
    .map_err(|_| AppError::TaskFailed)?
}

/// Ticks or unticks "Meaning" on a site (unticked: its meaning index goes).
#[tauri::command]
pub fn set_site_sense(app: AppHandle, state: State<'_, AppState>, id: String, on: bool) -> Result<crate::commands::SiteView, AppError> {
    if !on {
        if let Some(flag) = state.cancel_flag(&cancel_key(&id)) {
            flag.store(true, Ordering::Relaxed);
        }
    }
    let record = state.engine().set_site_sense(&id, on).map_err(AppError::Core)?;
    if on {
        wake(&app);
    }
    Ok(crate::commands::view(&state, record))
}

/// The pace of the computation: `normal` or `economy`.
#[tauri::command]
pub fn get_sense_pace(state: State<'_, AppState>) -> &'static str {
    match pace(&state) {
        Pace::Normal => "normal",
        Pace::Economy => "economy",
    }
}

#[tauri::command]
pub fn set_sense_pace(app: AppHandle, state: State<'_, AppState>, pace: String) -> &'static str {
    let mut settings = crate::settings::Settings::load(&state.settings_path);
    settings.sense_pace = Some(if pace == "economy" { "economy".to_owned() } else { "normal".to_owned() });
    settings.save(&state.settings_path);
    // The models are loaded again at the new pace, at the next round.
    *lock(&app.state::<SenseRuntime>().speed) = None;
    get_sense_pace(state)
}
