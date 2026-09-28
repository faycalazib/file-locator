//! "Open in the editor at this line" (lot 5.7). The code editors installed
//! are found in their usual folders; the user can also give a command of
//! their own with `{file}` and `{line}`. The program is started directly,
//! never through a shell: a file name cannot inject a command.

use std::path::PathBuf;
use std::process::Command;

use serde::Serialize;

/// An editor Prospector knows: where it installs itself, how it takes a line.
struct Known {
    id: &'static str,
    name: &'static str,
    /// (environment variable, path under it).
    places: &'static [(&'static str, &'static str)],
    args: &'static [&'static str],
}

const LOCAL: &str = "LOCALAPPDATA";
const PROGRAMS: &str = "ProgramFiles";
const PROGRAMS_X86: &str = "ProgramFiles(x86)";

const KNOWN: &[Known] = &[
    Known {
        id: "vscode",
        name: "Visual Studio Code",
        places: &[(LOCAL, r"Programs\Microsoft VS Code\Code.exe"), (PROGRAMS, r"Microsoft VS Code\Code.exe")],
        args: &["--goto", "{file}:{line}"],
    },
    Known {
        id: "vscodium",
        name: "VSCodium",
        places: &[(LOCAL, r"Programs\VSCodium\VSCodium.exe"), (PROGRAMS, r"VSCodium\VSCodium.exe")],
        args: &["--goto", "{file}:{line}"],
    },
    Known {
        id: "cursor",
        name: "Cursor",
        places: &[(LOCAL, r"Programs\cursor\Cursor.exe")],
        args: &["--goto", "{file}:{line}"],
    },
    Known {
        id: "notepadpp",
        name: "Notepad++",
        places: &[(PROGRAMS, r"Notepad++\notepad++.exe"), (PROGRAMS_X86, r"Notepad++\notepad++.exe")],
        args: &["-n{line}", "{file}"],
    },
    Known {
        id: "sublime",
        name: "Sublime Text",
        places: &[(PROGRAMS, r"Sublime Text\sublime_text.exe"), (PROGRAMS, r"Sublime Text 3\sublime_text.exe")],
        args: &["{file}:{line}"],
    },
];

/// The user's own command.
pub const CUSTOM: &str = "custom";

/// An editor found on this PC.
#[derive(Clone, Serialize)]
pub struct EditorInfo {
    pub id: &'static str,
    pub name: &'static str,
}

fn program(known: &Known) -> Option<PathBuf> {
    known.places.iter().find_map(|(var, rest)| {
        let path = PathBuf::from(std::env::var_os(var)?).join(rest);
        path.is_file().then_some(path)
    })
}

/// The known editors installed here, in the order above.
pub fn detect() -> Vec<EditorInfo> {
    KNOWN.iter().filter(|k| program(k).is_some()).map(|k| EditorInfo { id: k.id, name: k.name }).collect()
}

/// The editor used: the one chosen, else the first one found. `None`: no
/// editor (the custom command is empty, or nothing is installed).
pub fn resolve(chosen: Option<&str>, custom: &str) -> Option<(String, Vec<String>)> {
    match chosen {
        Some(CUSTOM) => {
            let mut parts = split_command(custom);
            (!parts.is_empty()).then(|| {
                let program = parts.remove(0);
                (program, parts)
            })
        }
        Some(id) => {
            let known = KNOWN.iter().find(|k| k.id == id)?;
            Some((program(known)?.to_string_lossy().into_owned(), known.args.iter().map(|a| (*a).to_owned()).collect()))
        }
        None => KNOWN.iter().find_map(|k| Some((program(k)?.to_string_lossy().into_owned(), k.args.iter().map(|a| (*a).to_owned()).collect()))),
    }
}

/// `"C:\Program Files\Vim\gvim.exe" +{line} "{file}"` → its parts; double
/// quotes group, and are removed.
pub fn split_command(command: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;
    for c in command.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    parts.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started {
        parts.push(current);
    }
    parts
}

/// The arguments with `{file}` and `{line}` filled in; without `{file}`,
/// the file is added at the end. Each part stays one argument, even when the
/// file name has spaces.
pub fn fill(args: &[String], file: &str, line: u32) -> Vec<String> {
    let line = line.max(1).to_string();
    let mut out: Vec<String> = args.iter().map(|a| a.replace("{file}", file).replace("{line}", &line)).collect();
    if !args.iter().any(|a| a.contains("{file}")) {
        out.push(file.to_owned());
    }
    out
}

pub fn open(program: &str, args: &[String], file: &str, line: u32) -> std::io::Result<()> {
    Command::new(program).args(fill(args, file, line)).spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|p| (*p).to_owned()).collect()
    }

    #[test]
    fn a_custom_command_is_split_on_spaces_outside_quotes() {
        assert_eq!(
            split_command(r#""C:\Program Files\Vim\gvim.exe" +{line} "{file}""#),
            strings(&[r"C:\Program Files\Vim\gvim.exe", "+{line}", "{file}"])
        );
        assert_eq!(split_command("  subl   {file}:{line} "), strings(&["subl", "{file}:{line}"]));
        assert_eq!(split_command(r#"app "" x"#), strings(&["app", "", "x"]), "an empty quoted argument is kept");
        assert!(split_command("   ").is_empty());
    }

    #[test]
    fn file_and_line_are_filled_in_one_argument_each() {
        let file = r"D:\Mes projets\app & co\main.rs";
        assert_eq!(fill(&strings(&["--goto", "{file}:{line}"]), file, 42), strings(&["--goto", r"D:\Mes projets\app & co\main.rs:42"]));
        assert_eq!(fill(&strings(&["-n{line}", "{file}"]), file, 7), strings(&["-n7", file]));
        // No {file}: added at the end. Line 0 becomes 1.
        assert_eq!(fill(&strings(&["-multiInst"]), file, 0), strings(&["-multiInst", file]));
    }

    #[test]
    fn an_empty_custom_command_is_no_editor() {
        assert!(resolve(Some(CUSTOM), "  ").is_none());
        assert_eq!(resolve(Some(CUSTOM), "notepad.exe"), Some(("notepad.exe".to_owned(), Vec::new())));
        assert!(resolve(Some("unknown"), "").is_none());
    }
}
