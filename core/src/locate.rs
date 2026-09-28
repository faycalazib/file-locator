//! Where Prospector keeps its settings and indexes, shared by the app and
//! `prospector-cli` (lot 7.1), so that both always use the same data:
//!
//! - portable mode (lot 6.8): a `portable` file next to the program puts
//!   everything in `ProspectorData` beside it;
//! - otherwise the folders Tauri uses for the app's identifier
//!   (`%APPDATA%\app.prospector.desktop` on Windows), where `settings.json`
//!   may name another data folder (Settings → Index folder).

use std::path::{Path, PathBuf};

/// The app's identifier (`tauri.conf.json`); portable copies add `.portable`.
pub const IDENTIFIER: &str = "app.prospector.desktop";
/// The file that turns the portable mode on.
pub const PORTABLE_MARKER: &str = "portable";
const PORTABLE_FOLDER: &str = "ProspectorData";

/// The data folder of a portable copy whose program is in `program_dir`.
pub fn portable_root(program_dir: &Path) -> Option<PathBuf> {
    program_dir.join(PORTABLE_MARKER).is_file().then(|| program_dir.join(PORTABLE_FOLDER))
}

/// The data folder of this program if it runs portable.
pub fn current_portable_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    portable_root(exe.parent()?)
}

/// The identifier of the running copy (single instance, lot 6.8).
pub fn identifier(portable: bool) -> String {
    if portable { format!("{IDENTIFIER}.portable") } else { IDENTIFIER.to_owned() }
}

/// The settings file and the default data folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    /// `Some`: portable mode, with its data folder.
    pub portable: Option<PathBuf>,
    pub settings_path: PathBuf,
    pub default_data_dir: PathBuf,
}

impl Location {
    /// Portable (`root`), or the app's folders for this user.
    pub fn new(portable: Option<PathBuf>) -> Option<Self> {
        Some(match portable {
            Some(root) => Self { settings_path: root.join("settings.json"), default_data_dir: root.join("data"), portable: Some(root) },
            None => Self {
                settings_path: dirs::config_dir()?.join(IDENTIFIER).join("settings.json"),
                default_data_dir: dirs::data_dir()?.join(IDENTIFIER),
                portable: None,
            },
        })
    }

    /// The data folder actually used: always the portable one; otherwise
    /// `PROSPECTOR_DATA_DIR` (development), then the saved choice, then the default.
    pub fn data_dir(&self, saved: Option<&str>) -> PathBuf {
        if self.portable.is_some() {
            return self.default_data_dir.clone();
        }
        if let Ok(dir) = std::env::var("PROSPECTOR_DATA_DIR") {
            if !dir.trim().is_empty() {
                return PathBuf::from(dir);
            }
        }
        saved.map_or_else(|| self.default_data_dir.clone(), PathBuf::from)
    }

    /// Saved searches, alerts and groups: this PC's, next to the settings,
    /// even when the data folder is shared (lot 7.3).
    pub fn personal_dir(&self) -> PathBuf {
        self.settings_path.parent().map_or_else(|| self.default_data_dir.clone(), Path::to_path_buf)
    }

    /// Settings read by the command line: the data folder and the OCR switch.
    pub fn read_settings(&self) -> (PathBuf, bool) {
        let settings: serde_json::Value =
            std::fs::read(&self.settings_path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let saved = settings.get("dataDir").and_then(|v| v.as_str());
        let ocr = settings.get("ocr").and_then(|v| v.as_bool()).unwrap_or(true);
        (self.data_dir(saved), ocr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_or_installed() {
        let dir = crate::test_tmp();
        assert_eq!(portable_root(dir.path()), None);
        std::fs::write(dir.path().join(PORTABLE_MARKER), "").unwrap();
        let root = portable_root(dir.path()).unwrap();
        assert_eq!(root, dir.path().join("ProspectorData"));
        let portable = Location::new(Some(root.clone())).unwrap();
        assert_eq!(portable.settings_path, root.join("settings.json"));
        assert_eq!(portable.data_dir(Some(r"D:\ailleurs")), root.join("data"), "portable ignores a saved folder");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&portable.settings_path, r#"{ "ocr": false }"#).unwrap();
        assert_eq!(portable.read_settings(), (root.join("data"), false));

        let installed = Location::new(None).unwrap();
        assert!(installed.settings_path.ends_with(Path::new(IDENTIFIER).join("settings.json")));
        assert_eq!(identifier(true), "app.prospector.desktop.portable");
    }
}
