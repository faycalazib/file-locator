//! The user's PATH (lot 7.1): Settings → "Command line in the PATH" adds
//! the program's folder, so that `prospector-cli` works in any new terminal.
//! Written in `HKCU\Environment` (this user only, no admin rights), with
//! its type kept (`REG_EXPAND_SZ`), then announced to Windows so that new
//! terminals see it. The uninstaller removes it (`prospector-cli
//! uninstall-cleanup`).

use std::path::Path;

/// The PATH entries, split on `;` (empty ones dropped).
fn entries(path: &str) -> Vec<&str> {
    path.split(';').filter(|e| !e.trim().is_empty()).collect()
}

/// Whether an entry is `dir` (case, final `\` and quotes ignored).
fn same(entry: &str, dir: &str) -> bool {
    let clean = |s: &str| s.trim().trim_matches('"').trim_end_matches(['\\', '/']).to_lowercase();
    clean(entry) == clean(dir)
}

/// `path` with `dir` at the end (unchanged if already there).
pub fn with_dir(path: &str, dir: &str) -> String {
    let mut list = entries(path);
    if !list.iter().any(|e| same(e, dir)) {
        list.push(dir);
    }
    list.join(";")
}

/// `path` without `dir`.
pub fn without_dir(path: &str, dir: &str) -> String {
    entries(path).into_iter().filter(|e| !same(e, dir)).collect::<Vec<_>>().join(";")
}

pub fn contains_dir(path: &str, dir: &str) -> bool {
    entries(path).iter().any(|e| same(e, dir))
}

#[cfg(windows)]
mod imp {
    use std::io;

    use winreg::enums::{RegType, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::{RegKey, RegValue};

    fn key(write: bool) -> io::Result<RegKey> {
        let access = if write { KEY_READ | KEY_WRITE } else { KEY_READ };
        RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Environment", access)
    }

    /// The user's PATH and its registry type.
    pub fn read() -> io::Result<(String, RegType)> {
        match key(false)?.get_raw_value("Path") {
            Ok(raw) => {
                let units: Vec<u16> = raw.bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
                let text = String::from_utf16_lossy(&units).trim_end_matches('\0').to_owned();
                Ok((text, raw.vtype))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok((String::new(), RegType::REG_EXPAND_SZ)),
            Err(e) => Err(e),
        }
    }

    pub fn write(path: &str, vtype: RegType) -> io::Result<()> {
        let bytes: Vec<u8> = path.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect();
        key(true)?.set_raw_value("Path", &RegValue { bytes, vtype })?;
        announce();
        Ok(())
    }

    /// Tells the Explorer (and the terminals it starts) that it changed.
    fn announce() {
        use windows::core::w;
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE};
        // SAFETY: a static wide string, no result buffer needed.
        unsafe {
            let _ = SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                WPARAM(0),
                LPARAM(w!("Environment").as_ptr() as isize),
                SMTO_ABORTIFHUNG,
                2000,
                None,
            );
        }
    }
}

/// Whether `dir` is in the user's PATH.
#[cfg(windows)]
pub fn has(dir: &Path) -> bool {
    imp::read().is_ok_and(|(path, _)| contains_dir(&path, &dir.to_string_lossy()))
}

/// Adds or removes `dir` in the user's PATH.
#[cfg(windows)]
pub fn set(dir: &Path, present: bool) -> std::io::Result<()> {
    let (path, vtype) = imp::read()?;
    let dir = dir.to_string_lossy();
    let changed = if present { with_dir(&path, &dir) } else { without_dir(&path, &dir) };
    if changed != path {
        imp::write(&changed, vtype)?;
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn has(_dir: &Path) -> bool {
    false
}

#[cfg(not(windows))]
pub fn set(_dir: &Path, _present: bool) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_remove_one_entry() {
        let dir = r"C:\Users\Léa\AppData\Local\Prospector";
        let path = r"C:\Windows;%USERPROFILE%\bin;";
        let added = with_dir(path, dir);
        assert_eq!(added, format!(r"C:\Windows;%USERPROFILE%\bin;{dir}"));
        assert_eq!(with_dir(&added, &format!("{dir}\\")), added, "already there");
        assert!(contains_dir(&added, &dir.to_uppercase()));
        assert_eq!(without_dir(&added, dir), r"C:\Windows;%USERPROFILE%\bin");
        assert_eq!(without_dir(r#""C:\Users\Léa\AppData\Local\Prospector";C:\x"#, dir), r"C:\x");
        assert_eq!(with_dir("", dir), dir);
    }
}
