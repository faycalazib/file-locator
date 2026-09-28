//! What Prospector is asked to open when it is launched (lot 5.7):
//! - `--in <folder>`: "Search with Prospector" from the Explorer;
//! - a `.prospector` file: saved results, opened by a double-click.
//!
//! One instance only: a second launch hands its arguments to the running one
//! (tauri-plugin-single-instance) and quits. Requests wait in `Launches` until
//! the UI takes them: the first launch's arrive before the UI listens.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::shortcut;
use crate::state::AppState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LaunchArg {
    Folder(PathBuf),
    Results(PathBuf),
}

/// A request as the UI receives it.
#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LaunchRequest {
    /// `site_id`: the indexed site holding the folder (indexed search), or
    /// none (live scan of the folder).
    Folder { path: String, site_id: Option<String> },
    Results { path: String },
}

#[derive(Default)]
pub struct Launches(Mutex<Vec<LaunchArg>>);

impl Launches {
    pub fn push(&self, args: Vec<LaunchArg>) {
        if let Ok(mut pending) = self.0.lock() {
            pending.extend(args);
        }
    }

    fn take(&self) -> Vec<LaunchArg> {
        self.0.lock().map(|mut p| std::mem::take(&mut *p)).unwrap_or_default()
    }
}

const RESULTS_EXTENSION: &str = "prospector";

/// `argv` of a launch (the program first). Unknown arguments are ignored.
pub fn parse(argv: &[String], cwd: &Path) -> Vec<LaunchArg> {
    let mut out = Vec::new();
    let mut args = argv.iter().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--in" {
            if let Some(folder) = args.next() {
                out.push(LaunchArg::Folder(absolute(&clean(folder), cwd)));
            }
        } else {
            let path = clean(arg);
            if path.extension().is_some_and(|e| e.eq_ignore_ascii_case(RESULTS_EXTENSION)) {
                out.push(LaunchArg::Results(absolute(&path, cwd)));
            }
        }
    }
    out
}

/// The Explorer quotes `%1`: for a drive, `"D:\"` reaches us as `D:"` (the
/// backslash escapes the quote). The quote goes, the drive gets its `\` back.
fn clean(arg: &str) -> PathBuf {
    let trimmed = arg.trim().trim_end_matches('"');
    let bytes = trimmed.as_bytes();
    if bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return PathBuf::from(format!("{trimmed}\\"));
    }
    PathBuf::from(trimmed)
}

fn absolute(path: &Path, cwd: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

/// Called with the arguments of a second launch: the window comes to the
/// front and the UI is told to take the requests.
pub fn second_launch<R: Runtime>(app: &AppHandle<R>, argv: Vec<String>, cwd: String) {
    let args = parse(&argv, Path::new(&cwd));
    shortcut::bring_to_front(app);
    if args.is_empty() {
        return;
    }
    app.state::<Launches>().push(args);
    let _ = app.emit("app://launch", ());
}

/// The requests waiting, for the UI. Folders that no longer exist are dropped.
#[tauri::command]
pub fn take_launch_requests(app: AppHandle) -> Vec<LaunchRequest> {
    let engine = app.state::<AppState>().engine();
    app.state::<Launches>()
        .take()
        .into_iter()
        .filter_map(|arg| match arg {
            LaunchArg::Folder(path) if path.is_dir() => {
                let path = path.to_string_lossy().into_owned();
                let site_id = engine.site_holding(&path).map(|s| s.id);
                Some(LaunchRequest::Folder { path, site_id })
            }
            LaunchArg::Results(path) if path.is_file() => {
                Some(LaunchRequest::Results { path: path.to_string_lossy().into_owned() })
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(args: &[&str]) -> Vec<String> {
        std::iter::once("prospector.exe").chain(args.iter().copied()).map(String::from).collect()
    }

    #[test]
    fn folder_from_the_explorer() {
        let cwd = Path::new(r"C:\Windows\System32");
        assert_eq!(parse(&argv(&["--in", r"D:\Mes clients\Dupont"]), cwd), vec![LaunchArg::Folder(r"D:\Mes clients\Dupont".into())]);
        // A drive: `"D:\"` arrives as `D:"`.
        assert_eq!(parse(&argv(&["--in", "D:\""]), cwd), vec![LaunchArg::Folder(r"D:\".into())]);
        assert_eq!(parse(&argv(&["--in", "D:"]), cwd), vec![LaunchArg::Folder(r"D:\".into())]);
        // `--in` with nothing after it, unknown arguments: ignored.
        assert!(parse(&argv(&["--in"]), cwd).is_empty());
        assert!(parse(&argv(&["--verbose", "notes.txt"]), cwd).is_empty());
    }

    #[test]
    fn results_file_by_double_click_or_relative() {
        let cwd = Path::new(r"D:\Exports");
        assert_eq!(parse(&argv(&[r"D:\Exports\contrat.prospector"]), cwd), vec![LaunchArg::Results(r"D:\Exports\contrat.prospector".into())]);
        assert_eq!(parse(&argv(&["Contrat.PROSPECTOR"]), cwd), vec![LaunchArg::Results(r"D:\Exports\Contrat.PROSPECTOR".into())]);
    }

    #[test]
    fn nothing_to_open_in_a_plain_launch() {
        assert!(parse(&argv(&[]), Path::new(r"C:\")).is_empty());
    }
}
