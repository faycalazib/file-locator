//! Documents inside archives and e-mails (lot 6.7): "Extract to…", opening
//! one with its own application, and the bytes of an attached image (its
//! picture in the results and the preview). A message of a mailbox is not
//! a file: only documents whose name has a known extension are extracted.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{CoreError, Result};
use crate::extract::{extract_raw, inner_name, split_inner};
use crate::kind::FileKind;

/// The family of a file inside a container (an attachment, an archive
/// entry): `None` for a file on disk and for a message of a mailbox.
pub fn inner_kind(path: &str) -> Option<FileKind> {
    let (_, inner) = split_inner(path);
    FileKind::from_path(Path::new(inner_name(inner?)))
}

/// Whether a result is a file inside a container that can be extracted.
pub fn is_extractable(path: &str) -> bool {
    inner_kind(path).is_some()
}

/// The name and the bytes of a document inside a container.
pub fn read(path: &str) -> Result<(String, Vec<u8>)> {
    let fail = || CoreError::NotExtractable { path: path.to_owned() };
    let (file, Some(inner)) = split_inner(path) else { return Err(fail()) };
    if !is_extractable(path) {
        return Err(fail());
    }
    let mut found = extract_raw(Path::new(file), &[inner]).map_err(|_| fail())?;
    let bytes = found.remove(inner).ok_or_else(fail)?;
    Ok((safe_name(inner_name(inner)), bytes))
}

/// "Extract to…": writes the document where the user chose in the save
/// dialog (which already asked before replacing a file).
pub fn extract_to(path: &str, target: &Path) -> Result<()> {
    let (_, bytes) = read(path)?;
    std::fs::write(target, bytes)?;
    Ok(())
}

/// Extracts the document into a new folder of `dir` (under its own name, so
/// that its application recognises it) and returns the file written. The
/// folder is emptied at the next start ([`clear_opened`]).
pub fn open_copy(path: &str, dir: &Path) -> Result<PathBuf> {
    let (name, bytes) = read(path)?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or_default();
    let folder = dir.join(format!("{stamp:x}"));
    std::fs::create_dir_all(&folder)?;
    let target = folder.join(name);
    std::fs::write(&target, bytes)?;
    Ok(target)
}

/// Removes the documents opened during the previous sessions. A file still
/// open in its application is left for next time.
pub fn clear_opened(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let _ = std::fs::remove_dir_all(entry.path());
    }
}

/// A name Windows accepts: no `<>:"/\|?*` nor control characters, no final
/// dot or space (names from Linux archives or e-mail attachments).
pub fn safe_name(name: &str) -> String {
    let cleaned: String = name.chars().map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { '_' } else { c }).collect();
    let trimmed = cleaned.trim_end_matches(['.', ' ']).trim_start();
    if trimmed.is_empty() { "document".to_owned() } else { trimmed.to_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::INNER_SEP;

    #[test]
    fn only_files_are_extractable() {
        assert!(is_extractable(&format!(r"D:\a.zip{INNER_SEP}x/y.pdf")));
        assert!(is_extractable(&format!(r"D:\m.pst{INNER_SEP}Inbox{INNER_SEP}Devis{INNER_SEP}devis.docx")));
        assert!(!is_extractable(&format!(r"D:\m.pst{INNER_SEP}Inbox{INNER_SEP}Re: devis")));
        assert!(!is_extractable(r"D:\a.pdf"));
        assert_eq!(inner_kind(&format!(r"D:\m.eml{INNER_SEP}photo.JPG")), Some(FileKind::Image));
    }

    #[test]
    fn names_windows_accepts() {
        assert_eq!(safe_name("rapport: v2?.txt"), "rapport_ v2_.txt");
        assert_eq!(safe_name("notes. "), "notes");
        assert_eq!(safe_name("..."), "document");
        assert_eq!(safe_name("عقد.docx"), "عقد.docx");
    }
}
