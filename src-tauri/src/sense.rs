//! Meaning-search module (Étape 8, lot 8.1), on the app side: its status,
//! downloading it from the release (progress `sense://progress`, cancel),
//! installing it from a file, removing it. It lives in this PC's personal
//! folder (`prospector_core::sense::module`).

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use prospector_core::sense::module::{self, ModuleStatus, MODULE_BYTES, MODULE_SHA256, MODULE_URL};
use prospector_core::CoreError;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

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
pub async fn install_sense_module(state: State<'_, AppState>, path: String) -> Result<ModuleStatus, AppError> {
    let target = dir(&state);
    tauri::async_runtime::spawn_blocking(move || module::install_zip(std::path::Path::new(&path), &target, None).map(|_| module::status(&target)))
        .await
        .map_err(|_| AppError::TaskFailed)?
        .map_err(AppError::Core)
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
    installed.map_err(AppError::Core)
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
pub fn remove_sense_module(state: State<'_, AppState>) -> Result<ModuleStatus, AppError> {
    let target = dir(&state);
    module::remove(&target).map_err(AppError::Core)?;
    Ok(module::status(&target))
}
