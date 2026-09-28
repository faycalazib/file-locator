//! Shared index (lot 7.3), on the app side: this run holds the lease of its
//! data folder (it indexes and watches the folders) or reads what another PC
//! keeps up to date. A thread renews the lease every `share::BEAT` seconds,
//! switches the role when it changes (the other PC closed, or this one slept
//! too long), and tells the interface (`share://status`, `sites://changed`).

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use prospector_core::share::{self, Lease, Role};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;
use crate::watch;

/// The lease of the current data folder.
#[derive(Default)]
pub struct ShareState {
    lease: Mutex<Option<(PathBuf, Lease)>>,
    role: Mutex<Option<Role>>,
}

/// What the rail shows.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareStatus {
    /// The data folder is on a network share.
    pub shared: bool,
    /// Another PC keeps it up to date (its name); none = this one.
    pub holder: Option<String>,
    /// This computer's name.
    pub pc: String,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn status(app: &AppHandle) -> ShareStatus {
    let engine = app.state::<AppState>().engine();
    ShareStatus { shared: share::is_network(engine.data_dir()), holder: engine.reader_of(), pc: share::computer_name() }
}

/// Claims (or renews) the lease of the current data folder and applies the
/// role. `first`: at startup, the folders are watched and caught up only
/// when this PC holds the lease.
fn tick(app: &AppHandle, first: bool) {
    let state = app.state::<AppState>();
    let shared = app.state::<ShareState>();
    let engine = state.engine();
    let data_dir = engine.data_dir().to_path_buf();
    let role = {
        let mut slot = lock(&shared.lease);
        // The data folder moved (Settings): the old lease is given back.
        if slot.as_ref().is_some_and(|(dir, _)| *dir != data_dir) {
            if let Some((_, old)) = slot.take() {
                old.release();
            }
        }
        let (_, lease) = slot.get_or_insert_with(|| (data_dir.clone(), Lease::new(&data_dir)));
        lease.claim(share::now())
    };
    let before = lock(&shared.role).replace(role.clone());
    if !first && before.as_ref() == Some(&role) {
        // Same role: a reader looks for the other PC's changes.
        if engine.refresh_shared() {
            let _ = app.emit("sites://changed", ());
        }
        return;
    }
    match role {
        Role::Maintainer => {
            engine.set_reader_of(None);
            watch::start_all(app.clone());
        }
        Role::Reader { pc } => {
            // Lost it (or never had it): stop watching and indexing.
            state.watchers.unwatch_all();
            state.cancel_all();
            engine.set_reader_of(Some(pc));
        }
    }
    if !first {
        let _ = app.emit("sites://changed", ());
    }
    let _ = app.emit("share://status", status(app));
}

/// At startup: first claim (watching only for the holder), then the thread.
pub fn start(app: &AppHandle) {
    tick(app, true);
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(share::BEAT));
        tick(&app, false);
    });
}

/// The data folder changed (Settings): claimed again at once.
pub fn restart(app: &AppHandle) {
    *lock(&app.state::<ShareState>().role) = None;
    tick(app, true);
    let _ = app.emit("share://status", status(app));
}

/// Prospector closes: the lease is free for the other PCs at once.
pub fn release(app: &AppHandle) {
    if let Some((_, lease)) = lock(&app.state::<ShareState>().lease).take() {
        lease.release();
    }
}

#[tauri::command]
pub fn get_share_status(app: AppHandle) -> ShareStatus {
    status(&app)
}
