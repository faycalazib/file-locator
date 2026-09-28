//! Tauri commands: the bridge between the UI and `prospector-core`.
//! Errors cross the bridge as `{ code, params }` (CoreError / AppError),
//! translated by the UI.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use std::path::Path;

use prospector_core::extract::split_inner;
use prospector_core::{
    live_scan as core_live_scan, relocate_data, CoreError, Engine, Hit, PreviewDoc, ScanSummary, ScanTarget,
    SavedSearch, SearchRequest, SearchResponse, SiteRecord,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::background;
use crate::editor::{self, EditorInfo};
use crate::explorer;
use crate::settings::Settings;
use crate::state::AppState;
use crate::shortcut::{self, ShortcutSlot, ShortcutStatus};
use crate::watch;

/// What the rail displays for a site (SiteRecord + live status).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteView {
    #[serde(flatten)]
    record: SiteRecord,
    /// `watching` (ready and watched) · `ready` · `indexing` · `error` · `empty` (never indexed)
    status: &'static str,
    error: Option<serde_json::Value>,
}

pub(crate) fn view(state: &AppState, record: SiteRecord) -> SiteView {
    let engine = state.engine();
    let (status, error) = if engine.is_busy(&record.id) {
        ("indexing", None)
    } else if let Some(path) = record.missing_roots.first() {
        ("error", serde_json::to_value(CoreError::RootUnavailable { path: path.clone() }).ok())
    } else if record.last_indexed.is_none() {
        ("empty", None)
    } else if state.watchers.is_watching(&record.id) {
        ("watching", None)
    } else {
        ("ready", None)
    };
    SiteView { record, status, error }
}

/// Errors that belong to the app shell, not the engine.
#[derive(Debug, Serialize)]
#[serde(tag = "code", content = "params", rename_all = "camelCase")]
pub enum AppError {
    Core(CoreError),
    OpenFailed { path: String },
    WriteFailed { path: String },
    /// Unreadable, too big or not UTF-8 text.
    ReadFailed { path: String },
    TaskFailed,
    /// The OS refused the global shortcut (used by another application).
    ShortcutUnavailable { shortcut: String },
    /// No code editor found, and no custom command.
    NoEditor,
    /// The editor could not be started (moved, uninstalled, wrong command).
    EditorFailed { program: String },
    /// The Explorer menu could not be written in the registry.
    ExplorerMenuFailed,
    /// Start with Windows could not be switched.
    AutostartFailed,
    /// The PATH could not be changed (lot 7.1).
    CliPathFailed,
}

impl From<CoreError> for AppError {
    fn from(e: CoreError) -> Self {
        AppError::Core(e)
    }
}

type CmdResult<T> = Result<T, AppError>;

/// Runs blocking engine work off the UI thread.
async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, CoreError> + Send + 'static) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|_| AppError::TaskFailed)?.map_err(AppError::Core)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: &'static str,
    data_dir: String,
    default_data_dir: String,
    updates_enabled: bool,
    /// Portable mode (lot 6.8): the data folder next to the program.
    portable_dir: Option<String>,
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: prospector_core::version(),
        data_dir: state.engine().data_dir().to_string_lossy().into_owned(),
        default_data_dir: state.default_data_dir.to_string_lossy().into_owned(),
        updates_enabled: state.updates_enabled,
        portable_dir: state.portable.as_ref().map(|p| p.to_string_lossy().into_owned()),
    }
}

/// The terms of a list file (lot 7.4), picked in the native open dialog.
#[tauri::command]
pub async fn read_term_list(path: String) -> CmdResult<prospector_core::terms::TermList> {
    blocking(move || prospector_core::terms::read_list(Path::new(&path))).await
}

/// Site groups (lot 7.2), in creation order (Ctrl+1…9).
#[tauri::command]
pub fn list_site_groups(state: State<'_, AppState>) -> Vec<prospector_core::SiteGroup> {
    state.engine().site_groups()
}

#[tauri::command]
pub fn create_site_group(state: State<'_, AppState>, name: String, site_ids: Vec<String>) -> CmdResult<prospector_core::SiteGroup> {
    Ok(state.engine().create_site_group(&name, site_ids)?)
}

#[tauri::command]
pub fn update_site_group(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    site_ids: Option<Vec<String>>,
) -> CmdResult<prospector_core::SiteGroup> {
    Ok(state.engine().update_site_group(&id, name.as_deref(), site_ids)?)
}

#[tauri::command]
pub fn remove_site_group(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    Ok(state.engine().remove_site_group(&id)?)
}

/// Portable mode (lot 6.8): the page of the new version, in the browser.
#[tauri::command]
pub fn open_releases_page(state: State<'_, AppState>) -> CmdResult<()> {
    let page = state.releases_page.clone().ok_or(AppError::OpenFailed { path: String::new() })?;
    tauri_plugin_opener::open_url(&page, None::<&str>).map_err(|_| AppError::OpenFailed { path: page })
}

#[tauri::command]
pub fn list_sites(state: State<'_, AppState>) -> Vec<SiteView> {
    state.engine().sites().into_iter().map(|s| view(&state, s)).collect()
}

#[tauri::command]
pub fn add_site(state: State<'_, AppState>, name: String, roots: Vec<String>) -> CmdResult<SiteView> {
    // A mapped drive (Z:) is kept under its network name, valid on every PC (lot 7.3).
    let roots = roots.iter().map(|r| prospector_core::share::universal(r)).collect();
    let record = state.engine().add_site(&name, roots)?;
    Ok(view(&state, record))
}

#[tauri::command]
pub async fn remove_site(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    let engine = state.engine();
    if !engine.is_busy(&id) {
        state.watchers.unwatch(&id);
    }
    blocking(move || engine.remove_site(&id)).await
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IndexFinished {
    pub site_id: String,
    pub site: Option<SiteView>,
    pub error: Option<serde_json::Value>,
}

/// Starts indexing in the background (an update: only changed files are
/// read, see `Engine::index_site`). Progress: `index://progress`; end:
/// `index://finished` (with the updated site, or the error code).
#[tauri::command]
pub fn index_site(app: AppHandle, state: State<'_, AppState>, id: String, excluded: Vec<String>) -> CmdResult<()> {
    if let Some(pc) = state.engine().reader_of() {
        return Err(CoreError::ReadOnly { pc }.into());
    }
    if state.engine().is_busy(&id) {
        return Err(CoreError::IndexBusy { id }.into());
    }
    std::thread::spawn(move || run_indexing(&app, &id, &excluded, false));
    Ok(())
}

/// Updates a site's index in the current thread, then starts watching its
/// folders. `quiet`: background run (startup, watcher), whose errors stay in
/// the rail (site status) instead of a notice.
pub(crate) fn run_indexing(app: &AppHandle, id: &str, excluded: &[String], quiet: bool) {
    let state = app.state::<AppState>();
    let engine = state.engine();
    let cancel = Arc::new(AtomicBool::new(false));
    state.set_cancel(id, cancel.clone());
    let result = engine.index_site(id, excluded, &cancel, &|p| {
        let _ = app.emit("index://progress", p);
    });
    if let Ok(mut map) = state.cancels.lock() {
        map.remove(id);
    }
    let finished = match result {
        Ok(record) => {
            if record.last_indexed.is_some() {
                watch::watch(app, &record);
            }
            // Files found by this update (changes made while closed too).
            if let Ok(news) = engine.check_alerts(id) {
                background::announce(app, news);
            }
            IndexFinished { site_id: id.to_owned(), site: Some(view(&state, record)), error: None }
        }
        Err(e) => IndexFinished {
            site_id: id.to_owned(),
            site: engine.site(id).ok().map(|r| view(&state, r)),
            error: if quiet { None } else { serde_json::to_value(e).ok() },
        },
    };
    let _ = app.emit("index://finished", finished);
}

#[tauri::command]
pub fn cancel_index(state: State<'_, AppState>, id: String) {
    if let Some(flag) = state.cancel_flag(&id) {
        flag.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
pub async fn search(state: State<'_, AppState>, site_ids: Vec<String>, request: SearchRequest) -> CmdResult<SearchResponse> {
    let engine = state.engine();
    blocking(move || engine.search(&site_ids, &request)).await
}

/// Preview with the same options as the search (Aa, ab, .*, typos).
#[tauri::command]
pub async fn preview(
    state: State<'_, AppState>,
    site_id: String,
    path: String,
    request: SearchRequest,
) -> CmdResult<PreviewDoc> {
    let engine = state.engine();
    blocking(move || engine.preview(&site_id, &path, &request)).await
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanHit {
    scan_id: String,
    hit: Hit,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanProgress {
    scan_id: String,
    scanned: usize,
    total: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanFinished {
    scan_id: String,
    summary: Option<ScanSummary>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveScanRequest {
    scan_id: String,
    site_ids: Vec<String>,
    excluded: Vec<String>,
    request: SearchRequest,
}

/// Search without the index: walks the sites' folders now. Emits
/// `scan://hit` for each find, `scan://progress`, then `scan://finished`.
#[tauri::command]
pub fn live_scan(app: AppHandle, state: State<'_, AppState>, scan: LiveScanRequest) -> CmdResult<()> {
    let engine = state.engine();
    let targets: Vec<ScanTarget> = scan
        .site_ids
        .iter()
        .filter_map(|id| engine.site(id).ok())
        .map(|s| ScanTarget { site_id: s.id, roots: s.roots.iter().map(PathBuf::from).collect() })
        .collect();
    let key = format!("scan:{}", scan.scan_id);
    let cancel = Arc::new(AtomicBool::new(false));
    state.set_cancel(&key, cancel.clone());
    let cancels = state.cancels.clone();

    std::thread::spawn(move || {
        let id = scan.scan_id.clone();
        let hit_app = app.clone();
        let progress_app = app.clone();
        let result = core_live_scan(
            &targets,
            &scan.excluded,
            &scan.request,
            &cancel,
            &|hit| {
                let _ = hit_app.emit("scan://hit", ScanHit { scan_id: id.clone(), hit });
            },
            &|scanned, total| {
                let _ = progress_app.emit("scan://progress", ScanProgress { scan_id: id.clone(), scanned, total });
            },
        );
        if let Ok(mut map) = cancels.lock() {
            map.remove(&key);
        }
        let finished = match result {
            Ok(summary) => ScanFinished { scan_id: scan.scan_id, summary: Some(summary), error: None },
            Err(e) => ScanFinished { scan_id: scan.scan_id, summary: None, error: serde_json::to_value(e).ok() },
        };
        let _ = app.emit("scan://finished", finished);
    });
    Ok(())
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>, scan_id: String) {
    if let Some(flag) = state.cancel_flag(&format!("scan:{scan_id}")) {
        flag.store(true, Ordering::Relaxed);
    }
}

/// A saved search as the UI sees it: its alert without the (long) list of
/// documents already seen.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedView {
    id: String,
    label: String,
    query: String,
    created: u64,
    settings: serde_json::Value,
    alert: Option<AlertView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertView {
    /// Announced, not looked at yet.
    fresh: Vec<String>,
}

impl From<SavedSearch> for SavedView {
    fn from(s: SavedSearch) -> Self {
        Self {
            id: s.id,
            label: s.label,
            query: s.query,
            created: s.created,
            settings: s.settings,
            alert: s.alert.map(|a| AlertView { fresh: a.fresh }),
        }
    }
}

#[tauri::command]
pub fn list_saved_searches(state: State<'_, AppState>) -> Vec<SavedView> {
    state.engine().saved_searches().into_iter().map(SavedView::from).collect()
}

/// Saves (or renames, if already saved) a search with its UI settings.
#[tauri::command]
pub fn save_search(
    state: State<'_, AppState>,
    label: String,
    query: String,
    settings: serde_json::Value,
) -> CmdResult<SavedView> {
    Ok(state.engine().save_search(&label, &query, settings)?.into())
}

/// "Alert me" on a saved search (lot 6.1). On: `request` is the engine
/// request the UI built from the saved settings; what it finds now is
/// remembered, never announced.
#[tauri::command]
pub async fn set_alert(
    state: State<'_, AppState>,
    id: String,
    request: Option<SearchRequest>,
    site_ids: Vec<String>,
) -> CmdResult<SavedView> {
    let engine = state.engine();
    let saved = blocking(move || match request {
        Some(request) => engine.enable_alert(&id, request, site_ids),
        None => engine.disable_alert(&id),
    })
    .await?;
    Ok(saved.into())
}

/// The user opened the saved search: its badge goes.
#[tauri::command]
pub fn mark_alert_read(state: State<'_, AppState>, id: String) -> CmdResult<SavedView> {
    Ok(state.engine().mark_alert_read(&id)?.into())
}

#[tauri::command]
pub fn remove_saved_search(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    Ok(state.engine().remove_saved_search(&id)?)
}

/// Moves catalog + indexes to another folder (Settings → Index folder).
#[tauri::command]
pub async fn set_data_dir(app: AppHandle, state: State<'_, AppState>, path: String) -> CmdResult<AppInfo> {
    // Portable: the data stays next to the program.
    if state.portable.is_some() {
        return Err(AppError::WriteFailed { path });
    }
    let current = state.engine();
    if current.sites().iter().any(|s| current.is_busy(&s.id)) {
        return Err(CoreError::IndexBusy { id: String::new() }.into());
    }
    let from = current.data_dir().to_path_buf();
    // A mapped drive is kept under its network name (lot 7.3).
    let to = PathBuf::from(prospector_core::share::universal(&path));
    let target = blocking(move || {
        // Documents opened from archives are not worth moving (lot 6.7).
        prospector_core::unpack::clear_opened(&opened_dir(&from));
        relocate_data(&from, &to)?;
        // relocate_data may have chosen a "Prospector" subfolder.
        Ok(if to.join("sites.json").exists() || !to.join("Prospector").exists() { to } else { to.join("Prospector") })
    })
    .await?;
    let engine = Engine::open_with(&target, &state.personal_dir)?;
    state.watchers.unwatch_all();
    state.replace_engine(engine);
    // The lease of the new folder decides who watches it (lot 7.3).
    crate::share::restart(&app);
    let mut settings = Settings::load(&state.settings_path);
    settings.data_dir = Some(target.to_string_lossy().into_owned());
    settings.save(&state.settings_path);
    Ok(AppInfo {
        version: prospector_core::version(),
        data_dir: target.to_string_lossy().into_owned(),
        default_data_dir: state.default_data_dir.to_string_lossy().into_owned(),
        updates_enabled: state.updates_enabled,
        portable_dir: None,
    })
}

/// OCR (lot 5.2): the user's choice, and what Windows can read.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrStatus {
    /// Switched on in Settings.
    enabled: bool,
    /// Windows has at least one recognizer language installed.
    available: bool,
    /// BCP 47 tags (`fr-FR`, `ar-SA`…).
    languages: Vec<String>,
}

fn ocr_status(state: &AppState) -> OcrStatus {
    let languages = prospector_core::extract::ocr::languages();
    OcrStatus {
        enabled: Settings::load(&state.settings_path).ocr.unwrap_or(true),
        available: !languages.is_empty(),
        languages,
    }
}

#[tauri::command]
pub fn get_ocr(state: State<'_, AppState>) -> OcrStatus {
    ocr_status(&state)
}

/// Switches OCR on or off. Switched on, every indexed site is updated in
/// the background: the images and PDFs read without OCR are read again (the
/// manifest remembers it), nothing else.
#[tauri::command]
pub fn set_ocr(app: AppHandle, state: State<'_, AppState>, enabled: bool) -> OcrStatus {
    prospector_core::extract::ocr::set_enabled(enabled);
    let mut settings = Settings::load(&state.settings_path);
    settings.ocr = Some(enabled);
    settings.save(&state.settings_path);
    if enabled {
        let sites: Vec<SiteRecord> = state.engine().sites().into_iter().filter(|s| s.last_indexed.is_some()).collect();
        std::thread::spawn(move || {
            for site in sites {
                run_indexing(&app, &site.id, &site.excluded, true);
            }
        });
    }
    ocr_status(&state)
}

#[tauri::command]
pub fn get_shortcut(app: AppHandle) -> ShortcutStatus {
    app.state::<ShortcutSlot>().get()
}

/// Changes the global shortcut (`""`: none). If the OS refuses it, the
/// previous one is restored and the error says so.
#[tauri::command]
pub fn set_shortcut(app: AppHandle, state: State<'_, AppState>, shortcut: String) -> CmdResult<ShortcutStatus> {
    let previous = app.state::<ShortcutSlot>().get();
    if let Err(e) = shortcut::apply(&app, &shortcut) {
        let _ = shortcut::apply(&app, &previous.shortcut);
        return Err(e);
    }
    let mut settings = Settings::load(&state.settings_path);
    settings.shortcut = Some(shortcut);
    settings.save(&state.settings_path);
    Ok(app.state::<ShortcutSlot>().get())
}

/// Search report: opens the print dialog of the window ("Save as PDF").
/// The UI has rendered the report just before (ReportView.svelte).
#[tauri::command]
pub fn print_report(app: AppHandle) -> CmdResult<()> {
    let window = app.get_webview_window("main").ok_or(AppError::TaskFailed)?;
    window.print().map_err(|_| AppError::TaskFailed)
}

/// Opens a file with its default program. For a document inside a ZIP or a
/// mailbox, the container itself is opened.
/// Opens a file with its application. A document inside an archive or an
/// e-mail (lot 6.7) is extracted first into the data folder (`opened`,
/// emptied at the next start); a message of a mailbox opens the mailbox.
#[tauri::command]
pub async fn open_file(state: State<'_, AppState>, path: String) -> CmdResult<()> {
    let (file, inner) = split_inner(&path);
    let target = if inner.is_some() && prospector_core::unpack::is_extractable(&path) {
        let dir = opened_dir(state.engine().data_dir());
        let inside = path.clone();
        blocking(move || prospector_core::unpack::open_copy(&inside, &dir)).await?
    } else {
        PathBuf::from(file)
    };
    tauri_plugin_opener::open_path(&target, None::<&str>).map_err(|_| AppError::OpenFailed { path: target.to_string_lossy().into_owned() })
}

/// Where the documents opened from archives and e-mails are extracted.
pub fn opened_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("opened")
}

/// "Extract to…" (lot 6.7): a document inside an archive or an e-mail,
/// written where the user chose in the save dialog.
#[tauri::command]
pub async fn extract_to(path: String, target: String) -> CmdResult<()> {
    blocking(move || prospector_core::unpack::extract_to(&path, Path::new(&target))).await
}

#[tauri::command]
pub fn reveal_file(path: String) -> CmdResult<()> {
    let (file, _) = split_inner(&path);
    tauri_plugin_opener::reveal_item_in_dir(Path::new(file)).map_err(|_| AppError::OpenFailed { path: file.to_owned() })
}

/// Writes an export (CSV / JSON) to the path chosen in the native save dialog.
#[tauri::command]
pub fn save_text_file(path: String, contents: String) -> CmdResult<()> {
    std::fs::write(&path, contents).map_err(|_| AppError::WriteFailed { path })
}

/// Saved results are small; anything bigger is not one of them.
const MAX_TEXT_FILE: u64 = 64 * 1024 * 1024;

/// Reads saved results (`.prospector`) chosen in the native open dialog.
#[tauri::command]
pub fn read_text_file(path: String) -> CmdResult<String> {
    let too_big = std::fs::metadata(&path).map(|m| m.len() > MAX_TEXT_FILE).unwrap_or(true);
    if too_big {
        return Err(AppError::ReadFailed { path });
    }
    std::fs::read_to_string(&path).map_err(|_| AppError::ReadFailed { path })
}

/// Windows integration (lot 5.7): Explorer menu and code editor.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationStatus {
    explorer_menu: bool,
    /// Portable mode: the Explorer menu is not offered.
    portable: bool,
    /// `prospector-cli.exe` is next to the program (lot 7.1; not offered
    /// in portable mode: the PATH is the PC's).
    cli_available: bool,
    /// Its folder is in the user's PATH.
    cli_in_path: bool,
    /// Known editors installed on this PC.
    editors: Vec<EditorInfo>,
    /// Chosen editor id, `""` = the first one found.
    editor: String,
    editor_command: String,
    /// An editor can be used (found, or a custom command).
    editor_ready: bool,
}

fn integration(state: &AppState) -> IntegrationStatus {
    let settings = Settings::load(&state.settings_path);
    let command = settings.editor_command.unwrap_or_default();
    IntegrationStatus {
        // Portable (lot 6.8): nothing written in the PC's registry.
        explorer_menu: state.portable.is_none() && settings.explorer_menu.unwrap_or_else(explorer::default_enabled),
        portable: state.portable.is_some(),
        cli_available: state.portable.is_none() && cli_dir().is_some(),
        cli_in_path: cli_dir().is_some_and(|dir| prospector_core::userpath::has(&dir)),
        editors: editor::detect(),
        editor_ready: editor::resolve(settings.editor.as_deref(), &command).is_some(),
        editor: settings.editor.unwrap_or_default(),
        editor_command: command,
    }
}

#[tauri::command]
pub fn get_integration(state: State<'_, AppState>) -> IntegrationStatus {
    integration(&state)
}

/// At startup and when the language changes: the menu, if on, is written
/// again with this program and the label in the interface language.
#[tauri::command]
pub fn sync_explorer_menu(state: State<'_, AppState>, label: String) -> IntegrationStatus {
    let status = integration(&state);
    if status.explorer_menu {
        let _ = explorer::register(&label);
    }
    status
}

/// The folder of `prospector-cli.exe`, when it is next to the program.
fn cli_dir() -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    dir.join(if cfg!(windows) { "prospector-cli.exe" } else { "prospector-cli" }).is_file().then_some(dir)
}

/// Settings → "Command line in the PATH" (lot 7.1): this user's PATH only.
#[tauri::command]
pub fn set_cli_path(state: State<'_, AppState>, enabled: bool) -> CmdResult<IntegrationStatus> {
    let dir = cli_dir().filter(|_| state.portable.is_none()).ok_or(AppError::CliPathFailed)?;
    prospector_core::userpath::set(&dir, enabled).map_err(|_| AppError::CliPathFailed)?;
    Ok(integration(&state))
}

#[tauri::command]
pub fn set_explorer_menu(state: State<'_, AppState>, enabled: bool, label: String) -> CmdResult<IntegrationStatus> {
    if state.portable.is_some() {
        return Err(AppError::ExplorerMenuFailed);
    }
    let written = if enabled { explorer::register(&label) } else { explorer::unregister() };
    written.map_err(|_| AppError::ExplorerMenuFailed)?;
    let mut settings = Settings::load(&state.settings_path);
    settings.explorer_menu = Some(enabled);
    settings.save(&state.settings_path);
    Ok(integration(&state))
}

/// `editor`: an id from `editors`, `custom`, or `""` (the first found).
#[tauri::command]
pub fn set_editor(state: State<'_, AppState>, editor: String, command: String) -> IntegrationStatus {
    let mut settings = Settings::load(&state.settings_path);
    settings.editor = (!editor.is_empty()).then_some(editor);
    settings.editor_command = Some(command);
    settings.save(&state.settings_path);
    integration(&state)
}

/// Opens a text or code file in the code editor at `line` (1-based).
#[tauri::command]
pub fn open_in_editor(state: State<'_, AppState>, path: String, line: u32) -> CmdResult<()> {
    let (file, inner) = split_inner(&path);
    if inner.is_some() || !Path::new(file).is_file() {
        return Err(AppError::OpenFailed { path });
    }
    let settings = Settings::load(&state.settings_path);
    let command = settings.editor_command.unwrap_or_default();
    let (program, args) = editor::resolve(settings.editor.as_deref(), &command).ok_or(AppError::NoEditor)?;
    editor::open(&program, &args, file, line).map_err(|_| AppError::EditorFailed { program })
}

/// SHA-256 and size of a file on disk ("Find copies", lot 5.8). Not for a
/// document inside an archive or a mailbox.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDigest {
    sha256: String,
    size: u64,
}

#[tauri::command]
pub async fn file_digest(path: String) -> CmdResult<FileDigest> {
    let (file, inner) = split_inner(&path);
    if inner.is_some() {
        return Err(AppError::ReadFailed { path });
    }
    let file = PathBuf::from(file);
    tauri::async_runtime::spawn_blocking(move || {
        let size = std::fs::metadata(&file)?.len();
        Ok::<_, std::io::Error>(FileDigest { sha256: prospector_core::disk::sha256_file(&file)?, size })
    })
    .await
    .map_err(|_| AppError::TaskFailed)?
    .map_err(|_| AppError::ReadFailed { path })
}

/// Keyword report (lot 6.4): occurrences of each term in each document.
#[tauri::command]
pub async fn keyword_report(
    state: State<'_, AppState>,
    docs: Vec<prospector_core::report::DocRef>,
    request: SearchRequest,
) -> CmdResult<prospector_core::report::KeywordReport> {
    let engine = state.engine();
    blocking(move || engine.keyword_report(&docs, &request)).await
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CopyProgress {
    copy_id: String,
    done: usize,
    total: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CopyFinished {
    copy_id: String,
    report: Option<prospector_core::copy::CopyReport>,
    error: Option<serde_json::Value>,
}

/// "Copy the files found" (lot 6.4) to a folder or a ZIP, in a thread:
/// `copy://progress`, then `copy://finished` with the report.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn copy_files(
    app: AppHandle,
    state: State<'_, AppState>,
    copy_id: String,
    paths: Vec<String>,
    target: String,
    zip: bool,
    keep_tree: bool,
    extract: bool,
) {
    let key = format!("copy:{copy_id}");
    let cancel = Arc::new(AtomicBool::new(false));
    state.set_cancel(&key, cancel.clone());
    let cancels = state.cancels.clone();
    std::thread::spawn(move || {
        let progress_app = app.clone();
        let id = copy_id.clone();
        let progress = move |done: usize, total: usize| {
            if done == total || done.is_multiple_of(20) {
                let _ = progress_app.emit("copy://progress", CopyProgress { copy_id: id.clone(), done, total });
            }
        };
        let target = PathBuf::from(target);
        let result = if zip {
            prospector_core::copy::copy_to_zip(&paths, &target, keep_tree, extract, &cancel, &progress)
        } else {
            prospector_core::copy::copy_to_folder(&paths, &target, keep_tree, extract, &cancel, &progress)
        };
        if let Ok(mut map) = cancels.lock() {
            map.remove(&key);
        }
        let finished = match result {
            Ok(report) => CopyFinished { copy_id, report: Some(report), error: None },
            Err(e) => CopyFinished { copy_id, report: None, error: serde_json::to_value(e).ok() },
        };
        let _ = app.emit("copy://finished", finished);
    });
}

#[tauri::command]
pub fn cancel_copy(state: State<'_, AppState>, copy_id: String) {
    if let Some(flag) = state.cancel_flag(&format!("copy:{copy_id}")) {
        flag.store(true, Ordering::Relaxed);
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DupProgressEvent {
    dup_id: String,
    #[serde(flatten)]
    progress: prospector_core::dupes::DupProgress,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DupFinished {
    dup_id: String,
    report: Option<prospector_core::dupes::DupReport>,
    error: Option<serde_json::Value>,
}

/// Duplicates in these sites (lot 6.5), in a thread: `dup://progress`,
/// then `dup://finished` with the groups.
#[tauri::command]
pub fn find_duplicates(
    app: AppHandle,
    state: State<'_, AppState>,
    dup_id: String,
    site_ids: Vec<String>,
    options: prospector_core::dupes::DupOptions,
) {
    let key = format!("dup:{dup_id}");
    let cancel = Arc::new(AtomicBool::new(false));
    state.set_cancel(&key, cancel.clone());
    let cancels = state.cancels.clone();
    let engine = state.engine();
    std::thread::spawn(move || {
        let progress_app = app.clone();
        let id = dup_id.clone();
        let result = engine.find_duplicates(&site_ids, options, &cancel, &|progress| {
            let _ = progress_app.emit("dup://progress", DupProgressEvent { dup_id: id.clone(), progress });
        });
        if let Ok(mut map) = cancels.lock() {
            map.remove(&key);
        }
        let finished = match result {
            Ok(report) => DupFinished { dup_id, report: Some(report), error: None },
            Err(e) => DupFinished { dup_id, report: None, error: serde_json::to_value(e).ok() },
        };
        let _ = app.emit("dup://finished", finished);
    });
}

#[tauri::command]
pub fn cancel_duplicates(state: State<'_, AppState>, dup_id: String) {
    if let Some(flag) = state.cancel_flag(&format!("dup:{dup_id}")) {
        flag.store(true, Ordering::Relaxed);
    }
}

/// Sends files to the Recycle Bin (lot 6.5; the UI asks first and always
/// keeps one file of a group). Documents inside an archive are refused.
/// Returns the paths that could not be moved.
#[tauri::command]
pub async fn trash_files(paths: Vec<String>) -> CmdResult<Vec<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .into_iter()
            .filter(|path| split_inner(path).1.is_some() || !Path::new(path).is_file() || trash::delete(path).is_err())
            .collect()
    })
    .await
    .map_err(|_| AppError::TaskFailed)
}

/// A thumbnail of an image file (lot 6.6) as a `data:` URL (the CSP allows
/// them), its longest side at most `size` pixels. An image inside an archive
/// or attached to an e-mail is extracted first (lot 6.7).
#[tauri::command]
pub async fn thumbnail(path: String, size: u32) -> CmdResult<String> {
    use base64::Engine as _;
    let size = size.clamp(16, 2048);
    let jpeg = blocking(move || {
        let (file, inner) = split_inner(&path);
        let (jpeg, _, _) = match inner {
            Some(_) => prospector_core::thumb::thumbnail_of_bytes(&prospector_core::unpack::read(&path)?.1, size)?,
            None => prospector_core::thumb::thumbnail(Path::new(file), size)?,
        };
        Ok(jpeg)
    })
    .await?;
    Ok(format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(jpeg)))
}

/// The boxes of the words matching the search on an image (lot 6.6).
#[tauri::command]
pub async fn image_matches(state: State<'_, AppState>, path: String, request: SearchRequest) -> CmdResult<Vec<prospector_core::ImageBox>> {
    let engine = state.engine();
    blocking(move || engine.image_matches(&path, &request)).await
}
