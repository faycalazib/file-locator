//! Portable mode (lot 6.8): a file named `portable` next to `prospector.exe`
//! puts everything in `ProspectorData` next to it — settings, indexes,
//! documents opened from archives and the WebView2 data (the interface's
//! preferences) — and nothing is written on the PC: no Explorer menu, no
//! start with Windows, no update installed (only a link to the new version).
//! When the drive gets another letter, the paths on it follow. Where the data
//! is: `prospector_core::locate` (shared with prospector-cli, lot 7.1).

use std::path::{Path, PathBuf};

use prospector_core::locate::Location;

use crate::settings::Settings;

/// The WebView2 data of a portable copy.
pub fn webview_dir(root: &Path) -> PathBuf {
    root.join("webview")
}

/// The drive got another letter since the last run: the paths on it are
/// moved (sites, indexes, saved searches), then the letter is recorded.
/// A failure is not fatal: the next start tries again.
pub fn follow_drive(location: &Location, settings: &mut Settings) {
    let Some(root) = &location.portable else { return };
    let Some(now) = prospector_core::portable::drive_of(root) else { return };
    let before = settings.portable_drive.clone();
    if before.as_deref().is_some_and(|b| b.eq_ignore_ascii_case(&now)) {
        return;
    }
    if let Some(before) = before {
        if prospector_core::portable::move_drive(&location.default_data_dir, &location.personal_dir(), &before, &now).is_err() {
            return;
        }
    }
    settings.portable_drive = Some(now);
    settings.save(&location.settings_path);
}

/// The page of the latest version, from the update address
/// (`…/releases/latest/download/latest.json` → `…/releases/latest`).
pub fn releases_page(updater: Option<&serde_json::Value>) -> Option<String> {
    let endpoint = updater?.get("endpoints")?.as_array()?.first()?.as_str()?;
    let (base, _) = endpoint.split_once("/releases/")?;
    Some(format!("{base}/releases/latest"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_of_the_latest_version() {
        let updater = serde_json::json!({ "endpoints": ["https://github.com/fa/prospector/releases/latest/download/latest.json"] });
        assert_eq!(releases_page(Some(&updater)).as_deref(), Some("https://github.com/fa/prospector/releases/latest"));
        assert_eq!(releases_page(None), None);
    }
}
