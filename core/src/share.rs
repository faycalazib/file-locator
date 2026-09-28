//! Shared index (lot 7.3): a data folder on a network share, used by several
//! PCs. One PC keeps it up to date (indexing, folder watching, catalog);
//! the others only read it. Who does is written in `maintainer.json`, a
//! lease renewed every [`BEAT`]: a lease not renewed for [`TTL`] (PC off,
//! Prospector closed without releasing it) is taken over by the next one.
//!
//! Also: telling a network folder, and turning a mapped drive (`Z:\…`) into
//! its network name (`\\server\share\…`), valid on every PC.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::fsutil::write_atomic;

/// How often the holder renews its lease (seconds).
pub const BEAT: u64 = 30;
/// A lease older than this is free (seconds).
pub const TTL: u64 = 120;

const FILE: &str = "maintainer.json";

/// Who keeps the index up to date.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Holder {
    /// Computer name, shown to the other PCs.
    pub pc: String,
    /// This run of Prospector (or of the command line).
    pub instance: String,
    /// Unix seconds of the last renewal.
    pub heartbeat: u64,
}

/// This PC's part in a data folder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "role")]
pub enum Role {
    /// Indexes, watches the folders, writes the catalog.
    Maintainer,
    /// Only reads what `pc` keeps up to date.
    Reader { pc: String },
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default()
}

/// This computer's name.
pub fn computer_name() -> String {
    std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_else(|_| "?".to_owned())
}

/// The lease of one data folder, held (or wanted) by one run.
pub struct Lease {
    file: PathBuf,
    me: Holder,
}

impl Lease {
    pub fn new(data_dir: &Path) -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or_default();
        let me = Holder { pc: computer_name(), instance: format!("{}-{}-{stamp:x}", computer_name(), std::process::id()), heartbeat: 0 };
        Self { file: data_dir.join(FILE), me }
    }

    /// For tests: another "PC".
    pub fn with_pc(data_dir: &Path, pc: &str, instance: &str) -> Self {
        Self { file: data_dir.join(FILE), me: Holder { pc: pc.to_owned(), instance: instance.to_owned(), heartbeat: 0 } }
    }

    fn read(&self) -> Option<Holder> {
        serde_json::from_slice(&std::fs::read(&self.file).ok()?).ok()
    }

    /// Takes the lease if it is free, stale or already ours (and renews it),
    /// else says who holds it. `now`: Unix seconds (injected by the tests).
    pub fn claim(&mut self, now: u64) -> Role {
        match self.read() {
            Some(other) if other.instance != self.me.instance && now.saturating_sub(other.heartbeat) < TTL => Role::Reader { pc: other.pc },
            _ => {
                self.me.heartbeat = now;
                let Ok(bytes) = serde_json::to_vec_pretty(&self.me) else { return Role::Maintainer };
                if write_atomic(&self.file, &bytes).is_err() {
                    // A folder we cannot write to: read only, from whoever.
                    return Role::Reader { pc: self.read().map(|h| h.pc).unwrap_or_default() };
                }
                // Two PCs writing at once: the last write wins, read it back.
                match self.read() {
                    Some(holder) if holder.instance != self.me.instance => Role::Reader { pc: holder.pc },
                    _ => Role::Maintainer,
                }
            }
        }
    }

    /// The computer holding a live lease, without claiming it (the command
    /// line reads a shared index held by someone else).
    pub fn holder(&self, now: u64) -> Option<String> {
        self.read().filter(|h| h.instance != self.me.instance && now.saturating_sub(h.heartbeat) < TTL).map(|h| h.pc)
    }

    /// Gives the lease back (Prospector closing), if it is still ours.
    pub fn release(&self) {
        if self.read().is_some_and(|h| h.instance == self.me.instance) {
            let _ = std::fs::remove_file(&self.file);
        }
    }

    pub fn pc(&self) -> &str {
        &self.me.pc
    }
}

/// Whether a folder is on another computer (UNC path or network drive).
pub fn is_network(path: &Path) -> bool {
    let text = path.to_string_lossy();
    if text.starts_with(r"\\") && !text.starts_with(r"\\?\") || text.starts_with(r"\\?\UNC\") {
        return true;
    }
    imp::is_remote_drive(path)
}

/// `Z:\Clients` → `\\server\share\Clients` when Z: is a network drive (its
/// letter may differ on the other PCs); any other path is returned as is.
pub fn universal(path: &str) -> String {
    imp::universal(path).unwrap_or_else(|| path.to_owned())
}

#[cfg(windows)]
mod imp {
    use std::path::{Component, Path, Prefix};

    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::NetworkManagement::WNet::{WNetGetUniversalNameW, UNIVERSAL_NAME_INFOW, UNIVERSAL_NAME_INFO_LEVEL};
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;

    const DRIVE_REMOTE: u32 = 4;

    fn letter(path: &Path) -> Option<char> {
        match path.components().next()? {
            Component::Prefix(p) => match p.kind() {
                Prefix::Disk(l) | Prefix::VerbatimDisk(l) => Some(char::from(l)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn is_remote_drive(path: &Path) -> bool {
        let Some(letter) = letter(path) else { return false };
        let root = HSTRING::from(format!("{letter}:\\"));
        // SAFETY: a NUL-terminated root path.
        unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) == DRIVE_REMOTE }
    }

    pub fn universal(path: &str) -> Option<String> {
        if !is_remote_drive(Path::new(path)) {
            return None;
        }
        let wide = HSTRING::from(path);
        let mut buffer = vec![0u8; 2048];
        let mut size = buffer.len() as u32;
        // SAFETY: the buffer and its size are given; the result is an
        // UNIVERSAL_NAME_INFOW whose string lives inside the buffer.
        unsafe {
            let done = WNetGetUniversalNameW(PCWSTR(wide.as_ptr()), UNIVERSAL_NAME_INFO_LEVEL, buffer.as_mut_ptr().cast(), &mut size);
            if done != windows::Win32::Foundation::NO_ERROR {
                return None;
            }
            let info = &*(buffer.as_ptr() as *const UNIVERSAL_NAME_INFOW);
            info.lpUniversalName.to_string().ok()
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn is_remote_drive(_: &std::path::Path) -> bool {
        false
    }

    pub fn universal(_: &str) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_holder_then_relay() {
        let dir = crate::test_tmp();
        let mut a = Lease::with_pc(dir.path(), "BUREAU", "a");
        let mut b = Lease::with_pc(dir.path(), "PORTABLE", "b");
        assert_eq!(a.claim(1000), Role::Maintainer);
        assert_eq!(b.claim(1010), Role::Reader { pc: "BUREAU".into() });
        // Renewed in time: still held.
        assert_eq!(a.claim(1000 + BEAT), Role::Maintainer);
        assert_eq!(b.claim(1000 + BEAT + TTL - 1), Role::Reader { pc: "BUREAU".into() });
        // Not renewed for TTL: the next one takes over; the old holder learns it.
        assert_eq!(b.claim(1000 + BEAT + TTL), Role::Maintainer);
        assert_eq!(a.claim(1000 + BEAT + TTL + 1), Role::Reader { pc: "PORTABLE".into() });
        // Released on closing: free at once.
        b.release();
        assert_eq!(a.claim(1000 + BEAT + TTL + 2), Role::Maintainer);
        b.release();
        assert!(dir.path().join(FILE).exists(), "only its holder releases it");
        assert_eq!(b.holder(1000 + BEAT + TTL + 3), Some("BUREAU".into()));
        assert_eq!(a.holder(1000 + BEAT + TTL + 3), None, "its own lease");
        assert_eq!(b.holder(1000 + BEAT + TTL + 2 + TTL), None, "stale");
    }

    #[test]
    fn network_paths() {
        assert!(is_network(Path::new(r"\\serveur\partage\Prospector")));
        assert!(is_network(Path::new(r"\\?\UNC\serveur\partage")));
        assert!(!is_network(Path::new(r"\\?\C:\Users")));
        assert_eq!(universal(r"C:\Users\Léa"), r"C:\Users\Léa", "a local disk stays as is");
    }
}
