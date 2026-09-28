mod background;
mod commands;
mod editor;
mod explorer;
mod launch;
mod portable;
mod sense;
mod settings;
mod share;
mod shortcut;
mod state;
mod watch;

use prospector_core::locate::{self, Location};
use prospector_core::Engine;
use tauri::{Manager, WebviewWindowBuilder};

use settings::Settings;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut context = tauri::generate_context!();
    // Portable (lot 6.8): its own identity, so that it never hands over to
    // (or blocks) a Prospector installed on the same PC (single instance).
    let portable_root = locate::current_portable_root();
    if portable_root.is_some() {
        context.config_mut().identifier = locate::identifier(true);
    }
    tauri::Builder::default()
        // First: a second launch hands its arguments over and quits at once.
        .plugin(tauri_plugin_single_instance::init(launch::second_launch))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(shortcut::plugin())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![background::HIDDEN_ARG]),
        ))
        .setup(move |app| {
            // Portable (lot 6.8): everything next to the program, and the
            // paths on the drive follow its letter before the engine opens.
            // Where the data is: shared with prospector-cli (lot 7.1).
            let location = match Location::new(portable_root.clone()) {
                Some(location) => location,
                None => {
                    let paths = app.path();
                    Location {
                        portable: None,
                        settings_path: paths.app_config_dir()?.join("settings.json"),
                        default_data_dir: paths.app_data_dir()?,
                    }
                }
            };
            let mut settings = Settings::load(&location.settings_path);
            portable::follow_drive(&location, &mut settings);
            let data_dir = location.data_dir(settings.data_dir.as_deref());
            let personal_dir = location.personal_dir();
            let Location { portable, settings_path, default_data_dir } = location;
            // The window (hidden until shown below) is made here: in portable
            // mode, its WebView2 data goes next to the program too.
            if let Some(config) = app.config().app.windows.first() {
                let mut window = WebviewWindowBuilder::from_config(app.handle(), config)?;
                if let Some(root) = &portable {
                    window = window.data_directory(portable::webview_dir(root));
                }
                window.build()?;
            }
            let engine = Engine::open_with(&data_dir, &personal_dir)?;
            // Documents opened from archives last session (lot 6.7).
            let opened = commands::opened_dir(&data_dir);
            std::thread::spawn(move || prospector_core::unpack::clear_opened(&opened));
            // Automatic updates only when the build carries an update key and a
            // server (`plugins.updater` in tauri.conf.json): without them the
            // plugin would refuse to start.
            let updater = app.config().plugins.0.get("updater").cloned();
            let updates_enabled = updater.is_some();
            if updates_enabled {
                app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
            }
            let mut state = AppState::new(engine, settings_path, default_data_dir, updates_enabled);
            state.releases_page = portable::releases_page(updater.as_ref());
            state.portable = portable;
            app.manage(state);
            // Meaning module (Étape 8): a removal asked before, an interrupted install or download.
            sense::cleanup(&app.state::<AppState>());
            app.manage(shortcut::ShortcutSlot::default());
            // `--in <folder>` (Explorer) or a `.prospector` file: kept for the UI.
            let launches = launch::Launches::default();
            let argv: Vec<String> = std::env::args().collect();
            launches.push(launch::parse(&argv, &std::env::current_dir().unwrap_or_default()));
            app.manage(launches);
            // Near the clock (lot 6.1). The window starts hidden (config) and
            // is shown, except for a start with Windows.
            app.manage(background::Texts::default());
            background::create_tray(app.handle())?;
            if !argv.iter().any(|a| a == background::HIDDEN_ARG) {
                shortcut::bring_to_front(app.handle());
            }
            // A shortcut taken by another app is reported in Settings, not fatal.
            let _ = shortcut::apply(app.handle(), settings.shortcut.as_deref().unwrap_or(shortcut::DEFAULT));
            prospector_core::extract::ocr::set_enabled(settings.ocr.unwrap_or(true));
            // Shared index (lot 7.3): only the holder of the lease watches the
            // folders and catches up the changes made while closed.
            app.manage(share::ShareState::default());
            share::start(app.handle());
            Ok(())
        })
        // Closing the window keeps Prospector near the clock when asked.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if background::keeps_running(&window.state::<AppState>()) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::list_sites,
            commands::add_site,
            commands::remove_site,
            commands::index_site,
            commands::cancel_index,
            commands::search,
            commands::preview,
            commands::live_scan,
            commands::cancel_scan,
            commands::list_saved_searches,
            commands::save_search,
            commands::remove_saved_search,
            commands::set_data_dir,
            commands::print_report,
            commands::get_ocr,
            commands::set_ocr,
            commands::get_shortcut,
            commands::set_shortcut,
            commands::open_file,
            commands::reveal_file,
            commands::save_text_file,
            commands::read_text_file,
            commands::get_integration,
            commands::sync_explorer_menu,
            commands::set_explorer_menu,
            commands::set_editor,
            commands::open_in_editor,
            commands::file_digest,
            commands::set_alert,
            commands::keyword_report,
            commands::copy_files,
            commands::cancel_copy,
            commands::find_duplicates,
            commands::cancel_duplicates,
            commands::trash_files,
            commands::thumbnail,
            commands::extract_to,
            commands::image_matches,
            commands::open_releases_page,
            commands::set_cli_path,
            commands::read_term_list,
            commands::list_site_groups,
            commands::create_site_group,
            commands::update_site_group,
            commands::remove_site_group,
            commands::mark_alert_read,
            background::sync_shell_texts,
            background::get_background,
            background::set_background,
            background::set_start_with_windows,
            launch::take_launch_requests,
            share::get_share_status,
            sense::sense_status,
            sense::install_sense_module,
            sense::download_sense_module,
            sense::cancel_sense_download,
            sense::remove_sense_module,
        ])
        .build(context)
        .expect("error while building Prospector")
        .run(|app, event| {
            // The lease of a shared index is free for the other PCs at once.
            if let tauri::RunEvent::Exit = event {
                share::release(app);
            }
        });
}
