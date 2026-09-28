//! "Search with Prospector" in the Explorer's right-click menu (lot 5.7), on
//! a folder, the background of an open folder and a drive. Written in
//! HKCU\Software\Classes: no admin rights, this user only.
//!
//! The app owns these keys (Settings → Explorer menu). It writes them again
//! at each start with the current program path and label (after an update or
//! a language change). The installers only remove them when uninstalling
//! (windows/hooks.nsh, windows/explorer-menu.wxs, same key names).
//!
//! Windows 11 lists them under "Show more options": the short menu requires
//! a signed MSIX package.

/// Key under `Software\Classes` → Explorer placeholder for the folder.
#[cfg(windows)]
const ENTRIES: [(&str, &str); 3] = [
    (r"Software\Classes\Directory\shell\Prospector", "%1"),
    (r"Software\Classes\Directory\Background\shell\Prospector", "%V"),
    (r"Software\Classes\Drive\shell\Prospector", "%1"),
];

/// The command the Explorer runs (`exe` quoted, the folder quoted).
#[cfg_attr(not(windows), allow(dead_code))]
fn command_line(exe: &str, placeholder: &str) -> String {
    format!("\"{exe}\" --in \"{placeholder}\"")
}

#[cfg(windows)]
mod imp {
    use std::io;

    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    use super::{command_line, ENTRIES};

    fn exe() -> io::Result<String> {
        Ok(std::env::current_exe()?.to_string_lossy().into_owned())
    }

    pub fn register(label: &str) -> io::Result<()> {
        let exe = exe()?;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        for (key, placeholder) in ENTRIES {
            let (entry, _) = hkcu.create_subkey(key)?;
            entry.set_value("MUIVerb", &label)?;
            entry.set_value("Icon", &format!("\"{exe}\",0"))?;
            let (command, _) = entry.create_subkey("command")?;
            command.set_value("", &command_line(&exe, placeholder))?;
        }
        Ok(())
    }

    pub fn unregister() -> io::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        for (key, _) in ENTRIES {
            match hkcu.delete_subkey_all(key) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn register(_label: &str) -> std::io::Result<()> {
        Ok(())
    }

    pub fn unregister() -> std::io::Result<()> {
        Ok(())
    }
}

pub use imp::{register, unregister};

/// Off by default in a dev build: the menu would point to target\debug.
pub fn default_enabled() -> bool {
    cfg!(windows) && !cfg!(debug_assertions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_quotes_the_program_and_the_folder() {
        assert_eq!(
            command_line(r"C:\Users\Léa\AppData\Local\Prospector\prospector.exe", "%V"),
            r#""C:\Users\Léa\AppData\Local\Prospector\prospector.exe" --in "%V""#
        );
    }
}
