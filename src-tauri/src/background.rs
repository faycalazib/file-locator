//! Prospector in the background (lot 6.1), so the alerts keep working:
//! - an icon near the clock (Open / Quit), always there;
//! - closing the window hides it instead of quitting, when "keep running"
//!   is on (by default: as soon as a saved search has an alert);
//! - optional start with Windows, hidden (`--background`).
//!
//! The alerts' notifications are sent from here too. Their texts and the
//! tray menu follow the interface language: the UI hands them over.

use std::sync::Mutex;

use prospector_core::AlertNews;
use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::NotificationExt;

use crate::commands::AppError;
use crate::settings::Settings;
use crate::shortcut;
use crate::state::AppState;

/// Argument of a start with Windows: the window stays hidden.
pub const HIDDEN_ARG: &str = "--background";
const TRAY_ID: &str = "main";

/// Texts in the interface language (English until the UI sends them).
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellTexts {
    pub tray_open: String,
    pub tray_quit: String,
    /// Notification title, with `{label}` (the saved search's name).
    pub alert_title: String,
}

impl Default for ShellTexts {
    fn default() -> Self {
        Self { tray_open: "Open Prospector".into(), tray_quit: "Quit".into(), alert_title: "New results: {label}".into() }
    }
}

#[derive(Default)]
pub struct Texts(pub Mutex<ShellTexts>);

impl Texts {
    fn get(&self) -> ShellTexts {
        self.0.lock().map(|t| t.clone()).unwrap_or_default()
    }
}

fn menu<R: Runtime>(app: &AppHandle<R>, texts: &ShellTexts) -> tauri::Result<Menu<R>> {
    let open = MenuItem::with_id(app, "open", &texts.tray_open, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", &texts.tray_quit, true, None::<&str>)?;
    Menu::with_items(app, &[&open, &quit])
}

/// The icon near the clock: a click opens the window, the menu quits.
pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let texts = app.state::<Texts>().get();
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Prospector")
        .menu(&menu(app, &texts)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => shortcut::bring_to_front(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                shortcut::bring_to_front(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// The UI's texts: the tray menu is rebuilt in the interface language.
#[tauri::command]
pub fn sync_shell_texts(app: AppHandle, texts: ShellTexts) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), menu(&app, &texts)) {
        let _ = tray.set_menu(Some(menu));
    }
    if let Ok(mut slot) = app.state::<Texts>().0.lock() {
        *slot = texts;
    }
}

/// Closing the window keeps Prospector running: the choice, or by default
/// as soon as an alert exists.
pub fn keeps_running(state: &AppState) -> bool {
    Settings::load(&state.settings_path)
        .background
        .unwrap_or_else(|| state.engine().saved_searches().iter().any(|s| s.alert.is_some()))
}

/// One notification per saved search with news; the UI updates its badges.
pub fn announce<R: Runtime>(app: &AppHandle<R>, news: Vec<AlertNews>) {
    if news.is_empty() {
        return;
    }
    let texts = app.state::<Texts>().get();
    for item in &news {
        let names: Vec<String> = item
            .found
            .iter()
            .take(3)
            .map(|path| path.rsplit(['\\', '/', '›']).next().unwrap_or(path).trim().to_owned())
            .collect();
        let more = if item.found.len() > 3 { format!(" (+{})", item.found.len() - 3) } else { String::new() };
        let _ = app
            .notification()
            .builder()
            .title(texts.alert_title.replace("{label}", &item.label))
            .body(format!("{}{more}", names.join(", ")))
            .show();
    }
    let _ = app.emit("alerts://news", news);
}

/// Background settings, for the Settings panel.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundStatus {
    keep_running: bool,
    /// Not chosen yet: follows the alerts.
    automatic: bool,
    start_with_windows: bool,
    /// Portable mode: start with Windows is not offered.
    portable: bool,
}

fn status(app: &AppHandle) -> BackgroundStatus {
    let state = app.state::<AppState>();
    BackgroundStatus {
        keep_running: keeps_running(&state),
        automatic: Settings::load(&state.settings_path).background.is_none(),
        start_with_windows: app.autolaunch().is_enabled().unwrap_or(false),
        portable: state.portable.is_some(),
    }
}

#[tauri::command]
pub fn get_background(app: AppHandle) -> BackgroundStatus {
    status(&app)
}

#[tauri::command]
pub fn set_background(app: AppHandle, keep_running: bool) -> BackgroundStatus {
    let state = app.state::<AppState>();
    let mut settings = Settings::load(&state.settings_path);
    settings.background = Some(keep_running);
    settings.save(&state.settings_path);
    status(&app)
}

#[tauri::command]
pub fn set_start_with_windows(app: AppHandle, enabled: bool) -> Result<BackgroundStatus, AppError> {
    // Portable (lot 6.8): nothing written in the PC's registry.
    if app.state::<AppState>().portable.is_some() {
        return Err(AppError::AutostartFailed);
    }
    let launcher = app.autolaunch();
    let done = if enabled { launcher.enable() } else { launcher.disable() };
    done.map_err(|_| AppError::AutostartFailed)?;
    Ok(status(&app))
}
