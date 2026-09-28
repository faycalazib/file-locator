//! Global shortcut (goal.md §4, Spotlight-like): from any application, it
//! brings Prospector to the front with the search box focused; pressed again
//! while Prospector is in front, it sends it back.
//! Default Ctrl+Shift+Space: Ctrl+Space switches the keyboard layout / IME
//! on Windows. The user picks another one (or none) in Settings.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::commands::AppError;

pub const DEFAULT: &str = "CmdOrCtrl+Shift+Space";

/// Current shortcut (`""` = none) and whether the OS accepted it.
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatus {
    pub shortcut: String,
    /// `false`: another application already uses it.
    pub active: bool,
}

#[derive(Default)]
pub struct ShortcutSlot(Mutex<ShortcutStatus>);

impl ShortcutSlot {
    pub fn get(&self) -> ShortcutStatus {
        self.0.lock().map(|s| s.clone()).unwrap_or_default()
    }

    fn set(&self, status: ShortcutStatus) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = status;
        }
    }
}

/// The plugin, with the handler shared by every shortcut.
pub fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                summon(app);
            }
        })
        .build()
}

/// Replaces the registered shortcut (`""`: none).
pub fn apply<R: Runtime>(app: &AppHandle<R>, shortcut: &str) -> Result<(), AppError> {
    let registry = app.global_shortcut();
    let _ = registry.unregister_all();
    let state = app.state::<ShortcutSlot>();
    if shortcut.is_empty() {
        state.set(ShortcutStatus { shortcut: String::new(), active: false });
        return Ok(());
    }
    let unavailable = || AppError::ShortcutUnavailable { shortcut: shortcut.to_owned() };
    let parsed: Shortcut = shortcut.parse().map_err(|_| unavailable())?;
    let active = registry.register(parsed).is_ok();
    state.set(ShortcutStatus { shortcut: shortcut.to_owned(), active });
    if active {
        Ok(())
    } else {
        Err(unavailable())
    }
}

/// Front and focused → minimized; otherwise shown, focused, search box ready.
fn summon<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let in_front = window.is_visible().unwrap_or(false)
        && window.is_focused().unwrap_or(false)
        && !window.is_minimized().unwrap_or(false);
    if in_front {
        let _ = window.minimize();
        return;
    }
    bring_to_front(app);
    let _ = app.emit("app://summon", ());
}

/// Shown, restored and focused (also on a second launch, launch.rs).
pub fn bring_to_front<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}
