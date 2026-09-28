//! What is read on the disk at search time (lot 5.8), never indexed: the
//! last-access date (Windows often does not keep it up to date), the
//! attributes (they change without the modification date moving) and the
//! digests (MD5 / SHA-256, computed on demand).

use std::fs::Metadata;
use std::io::Read;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use md5::Md5;
use sha2::Sha256;

use crate::error::{CoreError, Result};

/// Attribute bits (the Windows values).
pub const READ_ONLY: u8 = 0x1;
pub const HIDDEN: u8 = 0x2;
pub const SYSTEM: u8 = 0x4;

/// `readOnly`, `hidden`, `system` → bits; unknown names are ignored.
pub fn attribute_mask(names: &[String]) -> u8 {
    names.iter().fold(0, |mask, name| {
        mask | match name.as_str() {
            "readOnly" => READ_ONLY,
            "hidden" => HIDDEN,
            "system" => SYSTEM,
            _ => 0,
        }
    })
}

pub fn secs(time: std::io::Result<SystemTime>) -> u64 {
    time.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs())
}

/// Dates and attributes of one file, from its metadata.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// Unix seconds; the modification date where the system has none.
    pub created: u64,
    pub accessed: u64,
    pub attributes: u8,
}

impl Facts {
    pub fn from_meta(path: &Path, meta: &Metadata) -> Self {
        let modified = secs(meta.modified());
        let created = match secs(meta.created()) {
            0 => modified,
            c => c,
        };
        let accessed = match secs(meta.accessed()) {
            0 => modified,
            a => a,
        };
        Self { created, accessed, attributes: attributes(path, meta) }
    }

    pub fn read(path: &Path) -> Option<Self> {
        std::fs::metadata(path).ok().map(|meta| Self::from_meta(path, &meta))
    }
}

/// A dot name also counts as hidden, as for the crawl.
fn attributes(path: &Path, meta: &Metadata) -> u8 {
    let dot = path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'));
    #[cfg(windows)]
    let bits = {
        use std::os::windows::fs::MetadataExt;
        (meta.file_attributes() & 0x7) as u8
    };
    #[cfg(not(windows))]
    let bits = if meta.permissions().readonly() { READ_ONLY } else { 0 };
    bits | if dot { HIDDEN } else { 0 }
}

/// Inside `[from, to)` (either bound optional).
pub fn in_range(value: u64, from: Option<u64>, to: Option<u64>) -> bool {
    from.is_none_or(|f| value >= f) && to.is_none_or(|t| value < t)
}

/// A digest to look for, lowercase hex.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Digest {
    Md5(String),
    Sha256(String),
}

impl Digest {
    /// 32 hex digits = MD5, 64 = SHA-256 (spaces and case ignored); empty =
    /// none; anything else is refused.
    pub fn parse(input: &str) -> Result<Option<Self>> {
        let hex: String = input.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_lowercase();
        if hex.is_empty() {
            return Ok(None);
        }
        let invalid = || CoreError::InvalidHash { value: input.trim().to_owned() };
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(invalid());
        }
        match hex.len() {
            32 => Ok(Some(Self::Md5(hex))),
            64 => Ok(Some(Self::Sha256(hex))),
            _ => Err(invalid()),
        }
    }

    /// The file has this digest (unreadable = no).
    pub fn matches(&self, path: &Path) -> bool {
        match self {
            Self::Md5(hex) => hash_file::<Md5>(path).is_ok_and(|h| h == *hex),
            Self::Sha256(hex) => hash_file::<Sha256>(path).is_ok_and(|h| h == *hex),
        }
    }
}

/// SHA-256 of a file, lowercase hex ("find copies").
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    hash_file::<Sha256>(path)
}

/// Read in blocks: a big file is never loaded whole.
fn hash_file<H: sha2::Digest>(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = H::new();
    let mut buffer = vec![0u8; 256 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// What a request asks the disk: last access, attributes, digest.
#[derive(Clone, Debug, Default)]
pub struct DiskCheck {
    /// `[from, to)` on the last access.
    pub accessed: Option<(Option<u64>, Option<u64>)>,
    /// Every one of these bits is required.
    pub attributes: u8,
    pub digest: Option<Digest>,
}

impl DiskCheck {
    pub fn is_empty(&self) -> bool {
        self.accessed.is_none() && self.attributes == 0 && self.digest.is_none()
    }

    /// Dates and attributes only (the cheap part).
    pub fn accepts_facts(&self, facts: &Facts) -> bool {
        self.accessed.is_none_or(|(from, to)| in_range(facts.accessed, from, to))
            && facts.attributes & self.attributes == self.attributes
    }

    /// Everything, for a document of `file`. A document inside an archive
    /// or a mailbox has no digest of its own: refused when one is asked.
    pub fn accepts(&self, file: &Path, inner: bool, facts: Option<Facts>) -> bool {
        if self.digest.is_some() && inner {
            return false;
        }
        let facts = match facts {
            Some(facts) => facts,
            None if self.accessed.is_none() && self.attributes == 0 => Facts::default(),
            None => match Facts::read(file) {
                Some(facts) => facts,
                None => return false,
            },
        };
        self.accepts_facts(&facts) && self.digest.as_ref().is_none_or(|d| d.matches(file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digests_are_recognized_by_length() {
        let md5 = "D41D8CD98F00B204E9800998ECF8427E";
        assert_eq!(Digest::parse(md5).unwrap(), Some(Digest::Md5(md5.to_lowercase())));
        let sha = "e3b0c44298fc1c149afbf4c8996fb924 27ae41e4649b934ca495991b7852b855";
        assert!(matches!(Digest::parse(sha).unwrap(), Some(Digest::Sha256(h)) if h.len() == 64));
        assert_eq!(Digest::parse("  ").unwrap(), None);
        assert!(Digest::parse("abc").is_err());
        assert!(Digest::parse(&"z".repeat(32)).is_err());
    }

    #[test]
    fn digest_of_a_file() {
        let dir = crate::test_tmp();
        let empty = dir.path().join("vide.txt");
        std::fs::write(&empty, b"").unwrap();
        assert_eq!(sha256_file(&empty).unwrap(), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert!(Digest::parse("d41d8cd98f00b204e9800998ecf8427e").unwrap().unwrap().matches(&empty));
        assert!(!Digest::parse(&"0".repeat(64)).unwrap().unwrap().matches(&empty));
    }

    #[test]
    fn ranges_and_attributes() {
        assert!(in_range(10, Some(10), Some(11)));
        assert!(!in_range(11, Some(10), Some(11)), "the end is excluded");
        assert!(in_range(5, None, None));
        assert_eq!(attribute_mask(&["readOnly".into(), "system".into(), "x".into()]), READ_ONLY | SYSTEM);
        let check = DiskCheck { attributes: READ_ONLY, ..Default::default() };
        assert!(check.accepts_facts(&Facts { attributes: READ_ONLY | HIDDEN, ..Default::default() }));
        assert!(!check.accepts_facts(&Facts { attributes: HIDDEN, ..Default::default() }));
    }
}
