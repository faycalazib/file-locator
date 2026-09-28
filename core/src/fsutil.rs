//! Small file helpers shared by the JSON stores (catalog, manifests, saved
//! searches).

use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

/// Attempts at replacing a file: an antivirus scanning the new file can
/// refuse the rename for a moment ("Access is denied", BUG-022 / BUG-026).
const ATTEMPTS: u32 = 10;
const PAUSE: Duration = Duration::from_millis(100);

/// Writes `bytes` to `path` atomically: a temporary file, then a rename, so a
/// crash never leaves a half-written file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = Path::new(&tmp);
    std::fs::write(tmp, bytes)?;
    let mut attempt = 1;
    loop {
        match std::fs::rename(tmp, path) {
            Err(e) if e.kind() == ErrorKind::PermissionDenied && attempt < ATTEMPTS => {
                attempt += 1;
                std::thread::sleep(PAUSE);
            }
            result => return result,
        }
    }
}
