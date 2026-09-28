//! Portable mode (lot 6.8): Prospector and its data on a USB drive. The
//! drive may get another letter on another PC (`E:` here, `F:` there): the
//! paths on it are then rewritten everywhere they are kept — dig sites,
//! indexes, manifests, saved searches and alerts — from what is stored,
//! without reading any file again. Everything on the old letter is taken to
//! be on the drive.

use std::path::{Component, Path, Prefix};

use serde_json::Value;

use crate::error::Result;
use crate::fsutil::write_atomic;

/// The drive letter of a path (`"E:"`), none for a network path.
pub fn drive_of(path: &Path) -> Option<String> {
    match path.components().next()? {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => Some(format!("{}:", char::from(letter).to_ascii_uppercase())),
            _ => None,
        },
        _ => None,
    }
}

/// `path` on drive `to` if it is on drive `from` (`E:\Docs` → `F:\Docs`,
/// also `\\?\E:\…`); `None` otherwise. Letters are compared without case.
pub fn remap_path(path: &str, from: &str, to: &str) -> Option<String> {
    let (verbatim, rest) = match path.strip_prefix(r"\\?\") {
        Some(rest) => (r"\\?\", rest),
        None => ("", path),
    };
    let head = rest.get(..from.len())?;
    let tail = &rest[from.len()..];
    let on_drive = head.eq_ignore_ascii_case(from) && (tail.is_empty() || tail.starts_with(['\\', '/']));
    on_drive.then(|| format!("{verbatim}{to}{tail}"))
}

/// Every string and object key of a JSON value moved from `from` to `to`.
/// Returns whether something changed.
fn remap_json(value: &mut Value, from: &str, to: &str) -> bool {
    match value {
        Value::String(s) => match remap_path(s, from, to) {
            Some(moved) => {
                *s = moved;
                true
            }
            None => false,
        },
        // Every item is visited (no short circuit).
        Value::Array(items) => {
            let mut changed = false;
            for item in items {
                changed |= remap_json(item, from, to);
            }
            changed
        }
        Value::Object(map) => {
            let mut changed = false;
            let entries = std::mem::take(map);
            for (key, mut v) in entries {
                changed |= remap_json(&mut v, from, to);
                let key = remap_path(&key, from, to).inspect(|_| changed = true).unwrap_or(key);
                map.insert(key, v);
            }
            changed
        }
        _ => false,
    }
}

/// Rewrites a JSON file of the data folder if it holds paths on `from`.
fn remap_file(path: &Path, from: &str, to: &str) -> Result<()> {
    let Ok(bytes) = std::fs::read(path) else { return Ok(()) };
    let Ok(mut value) = serde_json::from_slice::<Value>(&bytes) else { return Ok(()) };
    if remap_json(&mut value, from, to) {
        // Manifests are big and written compact; the others are read by people.
        let compact = path.to_string_lossy().ends_with(".manifest.json");
        let bytes = if compact { serde_json::to_vec(&value)? } else { serde_json::to_vec_pretty(&value)? };
        write_atomic(path, &bytes)?;
    }
    Ok(())
}

/// What a change of drive letter rewrote.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DriveMove {
    /// Dig sites with at least one folder on the drive.
    pub sites: usize,
    /// Documents of their indexes.
    pub documents: usize,
}

/// The drive of the data folder was `from`, it is now `to`: rewrites the
/// paths on it, before the engine opens the data folder. Every step can be
/// run again (what is done already is left alone): the catalog is written
/// last, and the caller records the new letter only after success.
/// `personal_dir`: where the saved searches are (lot 7.3).
pub fn move_drive(data_dir: &Path, personal_dir: &Path, from: &str, to: &str) -> Result<DriveMove> {
    let mut done = DriveMove::default();
    let catalog = data_dir.join("sites.json");
    let Ok(bytes) = std::fs::read(&catalog) else { return Ok(done) };
    let sites: Value = serde_json::from_slice(&bytes)?;
    let indexes = data_dir.join("indexes");
    for site in sites.as_array().into_iter().flatten() {
        let Some(id) = site.get("id").and_then(Value::as_str) else { continue };
        // Documents are only on the folders of the site: a site with no
        // folder on the drive is not opened.
        let roots = site.get("roots").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str);
        if !roots.clone().any(|r| remap_path(r, from, to).is_some()) {
            continue;
        }
        done.sites += 1;
        let dir = indexes.join(id);
        if dir.is_dir() {
            done.documents += crate::index::remap_paths(&dir, &|p| remap_path(p, from, to))?;
        }
        remap_file(&indexes.join(format!("{id}.manifest.json")), from, to)?;
        // The meaning index (Étape 8) and what it has computed.
        let sense = indexes.join(format!("{id}.sense"));
        if sense.is_dir() {
            crate::sense::store::SenseIndex::open(&sense, false)?.remap_paths(&|p| remap_path(p, from, to))?;
        }
        remap_file(&indexes.join(format!("{id}.sense.json")), from, to)?;
    }
    remap_file(&personal_dir.join("saved-searches.json"), from, to)?;
    remap_file(&catalog, from, to)?;
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_letters() {
        assert_eq!(drive_of(Path::new(r"e:\Prospector\prospector.exe")), Some("E:".to_owned()));
        assert_eq!(drive_of(Path::new(r"\\?\F:\Prospector")), Some("F:".to_owned()));
        assert_eq!(drive_of(Path::new(r"\\serveur\partage\Prospector")), None);
    }

    #[test]
    fn only_paths_on_the_old_drive_move() {
        assert_eq!(remap_path(r"E:\Docs\a.pdf", "E:", "F:").as_deref(), Some(r"F:\Docs\a.pdf"));
        assert_eq!(remap_path(r"e:\Docs", "E:", "F:").as_deref(), Some(r"F:\Docs"));
        assert_eq!(remap_path(r"\\?\E:\Docs", "E:", "F:").as_deref(), Some(r"\\?\F:\Docs"));
        assert_eq!(remap_path("E:", "E:", "F:").as_deref(), Some("F:"));
        assert_eq!(remap_path(r"C:\Docs", "E:", "F:"), None);
        assert_eq!(remap_path("E:tude", "E:", "F:"), None, "not a path on E:");
        assert_eq!(remap_path("contrat", "E:", "F:"), None);
    }

    #[test]
    fn json_strings_and_keys_move() {
        let mut v = serde_json::json!({ r"E:\a.txt": { "size": 1 }, "roots": [r"E:\Docs", r"C:\Autre"], "q": "E:tude" });
        assert!(remap_json(&mut v, "E:", "F:"));
        assert_eq!(v, serde_json::json!({ r"F:\a.txt": { "size": 1 }, "roots": [r"F:\Docs", r"C:\Autre"], "q": "E:tude" }));
        assert!(!remap_json(&mut v, "E:", "F:"), "nothing left to move");
    }
}
