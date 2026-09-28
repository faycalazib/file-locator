//! The meaning-search module on disk (lot 8.1): installed from its ZIP
//! (downloaded, or a file chosen by the user), checked file by file against
//! its manifest, and removed. It lives in this PC's personal folder
//! (`modules/sense`): it follows the portable drive, and it is not shared
//! with the other PCs of a shared index.

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{CoreError, Result};

/// Where the module of this release is published (the GitHub release of the
/// Prospector repository, lot 4). `None` until the repository exists: only
/// "Install from a file" is offered.
pub const MODULE_URL: Option<&str> = None;
/// SHA-256 of that ZIP (`scripts/sense-module.py` prints it).
pub const MODULE_SHA256: &str = "1625f526f80a844b00bcc9118976be67f111b575ea2dc764a22a597ba3d72411";
/// Its size, for the download progress and the confirmation (bytes).
pub const MODULE_BYTES: u64 = 80_437_000;
/// Module format this build reads.
pub const MODULE_VERSION: u32 = 1;

/// A ZIP bigger than this is not our module.
const MAX_ZIP_BYTES: u64 = 512 * 1024 * 1024;
const REMOVE_FLAG: &str = "remove-at-start";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub module: String,
    pub version: u32,
    pub model: String,
    pub dimensions: usize,
    pub onnxruntime: String,
    /// SHA-256 of every file.
    pub files: std::collections::BTreeMap<String, String>,
}

/// What the Settings show.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleStatus {
    pub installed: bool,
    pub model: Option<String>,
    /// Bytes on disk.
    pub size: u64,
    /// Can be downloaded from the release (`MODULE_URL` set).
    pub downloadable: bool,
    /// Download size.
    pub download_size: u64,
}

pub fn module_dir(personal_dir: &Path) -> PathBuf {
    personal_dir.join("modules").join("sense")
}

fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Sense { message: message.into() }
}

/// The installed module's manifest, if it is complete and of this format.
pub fn installed(dir: &Path) -> Option<Manifest> {
    if dir.join(REMOVE_FLAG).exists() {
        return None;
    }
    let manifest: Manifest = serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).ok()?).ok()?;
    let complete = manifest.module == "sense" && manifest.version == MODULE_VERSION && manifest.files.keys().all(|f| dir.join(f).is_file());
    complete.then_some(manifest)
}

pub fn status(dir: &Path) -> ModuleStatus {
    let manifest = installed(dir);
    let size = manifest
        .as_ref()
        .map(|m| m.files.keys().filter_map(|f| std::fs::metadata(dir.join(f)).ok()).map(|m| m.len()).sum())
        .unwrap_or(0);
    ModuleStatus {
        installed: manifest.is_some(),
        model: manifest.map(|m| m.model),
        size,
        downloadable: MODULE_URL.is_some(),
        download_size: MODULE_BYTES,
    }
}

/// Installs the module from its ZIP. `expected`: the SHA-256 the ZIP must
/// have (a download); a file chosen by the user is checked through its
/// manifest instead. Unpacked aside, checked file by file, then put in place.
pub fn install_zip(zip_path: &Path, dir: &Path, expected: Option<&str>) -> Result<Manifest> {
    let size = std::fs::metadata(zip_path)?.len();
    if size > MAX_ZIP_BYTES {
        return Err(invalid("not a Prospector meaning module (too large)"));
    }
    let bytes = std::fs::read(zip_path)?;
    if let Some(expected) = expected {
        if !sha256_hex(&bytes).eq_ignore_ascii_case(expected) {
            return Err(invalid("the downloaded module is damaged (wrong SHA-256)"));
        }
    }
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| invalid("not a ZIP file"))?;
    let read = |archive: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>, name: &str| -> Result<Vec<u8>> {
        let mut entry = archive.by_name(name).map_err(|_| invalid(format!("{name} is missing")))?;
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        Ok(data)
    };
    let manifest: Manifest =
        serde_json::from_slice(&read(&mut archive, "manifest.json")?).map_err(|_| invalid("not a Prospector meaning module (no manifest)"))?;
    if manifest.module != "sense" || manifest.version != MODULE_VERSION {
        return Err(invalid(format!("module version {} is not the one this Prospector reads ({MODULE_VERSION})", manifest.version)));
    }

    let parent = dir.parent().ok_or_else(|| invalid("no folder for the module"))?;
    std::fs::create_dir_all(parent)?;
    let staging = dir.with_extension("installing");
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;
    for (name, sha) in &manifest.files {
        // Names are plain file names: nothing written outside the folder.
        if name.contains(['/', '\\']) || name.starts_with('.') {
            return Err(invalid(format!("unexpected file name {name}")));
        }
        let data = read(&mut archive, name)?;
        if !sha256_hex(&data).eq_ignore_ascii_case(sha) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(invalid(format!("{name} is damaged (wrong SHA-256)")));
        }
        std::fs::write(staging.join(name), data)?;
    }
    std::fs::write(staging.join("manifest.json"), serde_json::to_vec_pretty(&manifest)?)?;

    // The old module goes aside (its DLL may be in use: then it is removed at the next start).
    if dir.exists() {
        let old = dir.with_extension("old");
        let _ = std::fs::remove_dir_all(&old);
        std::fs::rename(dir, &old)?;
        let _ = std::fs::remove_dir_all(&old);
    }
    std::fs::rename(&staging, dir)?;
    Ok(manifest)
}

/// Removes the module. Its files may be in use (ONNX Runtime loaded in this
/// process): then it is marked, hidden at once, and removed at the next start.
pub fn remove(dir: &Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    if std::fs::remove_dir_all(dir).is_err() {
        std::fs::write(dir.join(REMOVE_FLAG), b"")?;
    }
    Ok(())
}

/// At startup: finishes a removal asked before, and cleans an interrupted install.
pub fn cleanup(dir: &Path) {
    if dir.join(REMOVE_FLAG).exists() {
        let _ = std::fs::remove_dir_all(dir);
    }
    for leftover in [dir.with_extension("installing"), dir.with_extension("old")] {
        let _ = std::fs::remove_dir_all(leftover);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn module_zip(path: &Path, tamper: bool) {
        let files = [("model.onnx", b"model".as_slice()), ("tokenizer.json", b"{}".as_slice())];
        let manifest = Manifest {
            module: "sense".into(),
            version: MODULE_VERSION,
            model: "test".into(),
            dimensions: 384,
            onnxruntime: "1.28.0".into(),
            files: files.iter().map(|(n, d)| ((*n).to_owned(), sha256_hex(d))).collect(),
        };
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        for (name, data) in files {
            zip.start_file(name, options).unwrap();
            zip.write_all(if tamper && name == "model.onnx" { b"other" } else { data }).unwrap();
        }
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(&serde_json::to_vec(&manifest).unwrap()).unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn install_check_and_remove() {
        let tmp = crate::test_tmp();
        let dir = module_dir(tmp.path());
        let good = tmp.path().join("good.zip");
        module_zip(&good, false);
        assert!(!status(&dir).installed);
        // A download must have the expected SHA-256.
        assert!(install_zip(&good, &dir, Some("00")).is_err());
        let manifest = install_zip(&good, &dir, None).unwrap();
        assert_eq!(manifest.model, "test");
        let s = status(&dir);
        assert!(s.installed && s.size == 7);
        // A damaged file is refused, and the installed module stays.
        let bad = tmp.path().join("bad.zip");
        module_zip(&bad, true);
        assert!(install_zip(&bad, &dir, None).is_err());
        assert!(status(&dir).installed);
        // Not a module at all.
        std::fs::write(tmp.path().join("x.zip"), b"hello").unwrap();
        assert!(install_zip(&tmp.path().join("x.zip"), &dir, None).is_err());
        remove(&dir).unwrap();
        cleanup(&dir);
        assert!(!status(&dir).installed && !dir.exists());
    }
}
