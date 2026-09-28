//! App settings stored in the OS config folder (AppData on Windows).
//! The index data itself lives in `data_dir`, which the user can move
//! (Settings → Index folder).

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Folder of the catalog and indexes; `None` = default (AppData).
    pub data_dir: Option<String>,
    /// Global shortcut; `None` = default, `""` = none.
    #[serde(default)]
    pub shortcut: Option<String>,
    /// Read the text of images and scanned PDFs (Windows OCR); `None` = on.
    #[serde(default)]
    pub ocr: Option<bool>,
    /// "Search with Prospector" in the Explorer menu; `None` = on in the
    /// installed app, off in a dev build (explorer.rs).
    #[serde(default)]
    pub explorer_menu: Option<bool>,
    /// Code editor (`vscode`, `notepadpp`…, `custom`); `None` = the first found.
    #[serde(default)]
    pub editor: Option<String>,
    /// Custom editor command, with `{file}` and `{line}`.
    #[serde(default)]
    pub editor_command: Option<String>,
    /// Closing the window keeps Prospector running (lot 6.1); `None` = as
    /// soon as a saved search has an alert.
    #[serde(default)]
    pub background: Option<bool>,
    /// Portable mode (lot 6.8): the drive letter of the last run (`"E:"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portable_drive: Option<String>,
    /// Meaning index (Étape 8): `normal` or `economy`; none = normal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sense_pace: Option<String>,
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(self) {
            let _ = std::fs::write(path, bytes);
        }
    }
}
