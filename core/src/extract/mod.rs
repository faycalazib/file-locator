//! Content extractors: one module per format family (goal.md §2).
//! Étape 1: text / code / e-mail (.eml), PDF, Word (.docx).
//! Étape 2: Excel (xlsx, xls, ods), PowerPoint (pptx), ZIP (recursive),
//! Outlook (.msg, .pst).

mod archive;
pub mod ocr;
mod pdf_ocr;
mod doc;
mod docx;
mod epub;
mod mail;
mod odf;
mod office;
mod pdf;
mod ppt;
mod rtf;
mod text;

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;

use crate::kind::FileKind;

/// Files above this size are skipped (content extraction would be too slow
/// or the file is not a document). Archives and mailboxes are streamed from
/// disk and not concerned.
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Why a file could not be read. Reported as counters to the UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SkipReason {
    /// Format recognized but not supported (old .doc/.ppt, RAR, 7z…).
    Unsupported,
    TooLarge,
    /// Looks binary although the extension says text.
    Binary,
    /// Password-protected document.
    Encrypted,
    /// Damaged or unreadable file.
    Corrupt,
    /// Locked by another program or access denied.
    Unreadable,
}

/// One searchable document. Container files (ZIP, PST) produce several,
/// each with its `inner` path (`folder/contract.pdf`, `Inbox › Subject`).
#[derive(Clone, Debug)]
pub struct ExtractedDoc {
    pub inner: Option<String>,
    pub kind: FileKind,
    /// Size of the inner document (the file size is used otherwise).
    pub size: Option<u64>,
    /// Unix seconds of the inner document (the file date is used otherwise).
    pub modified: Option<u64>,
    pub text: String,
}

impl ExtractedDoc {
    fn whole(kind: FileKind, text: String) -> Self {
        Self { inner: None, kind, size: None, modified: None, text }
    }
}

/// Separator between a container file and a document inside it.
pub const INNER_SEP: &str = " › ";

fn ext(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default()
}

/// Whether this family can be read at the current step (by extension).
pub fn is_supported(path: &Path, kind: FileKind) -> bool {
    let e = ext(path);
    match kind {
        FileKind::Text | FileKind::Code | FileKind::Pdf => true,
        FileKind::Word => matches!(e.as_str(), "docx" | "docm" | "doc" | "rtf" | "odt"),
        FileKind::Excel => matches!(e.as_str(), "xlsx" | "xlsm" | "xls" | "xlsb" | "ods"),
        FileKind::Powerpoint => matches!(e.as_str(), "pptx" | "pptm" | "ppt" | "odp"),
        FileKind::Archive => archive::is_supported(path),
        // Images are read by OCR (Windows), when switched on in Settings.
        FileKind::Image => ocr::enabled(),
        FileKind::Email => matches!(e.as_str(), "eml" | "msg" | "pst" | "ost" | "mbox"),
    }
}

/// Every searchable document of a file.
pub fn extract_docs(path: &Path, kind: FileKind) -> Result<Vec<ExtractedDoc>, SkipReason> {
    if !is_supported(path, kind) {
        return Err(SkipReason::Unsupported);
    }
    let e = ext(path);
    // Containers are read from disk, entry by entry.
    if kind == FileKind::Archive {
        return archive::archive_file(path);
    }
    // Mailboxes are streamed from disk, message by message.
    if matches!(e.as_str(), "pst" | "ost") {
        return mail::pst(path).map(|docs| docs.into_iter().map(clean_doc).collect());
    }
    if e == "mbox" {
        return mail::mbox(path).map(|docs| docs.into_iter().map(clean_doc).collect());
    }
    let meta = std::fs::metadata(path).map_err(|_| SkipReason::Unreadable)?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(SkipReason::TooLarge);
    }
    let bytes = std::fs::read(path).map_err(|_| SkipReason::Unreadable)?;
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    docs_from_bytes(&bytes, kind, name)
}

/// The documents of one file in memory: one, or for a message (.msg, .eml)
/// the message followed by its attachments (lot 5.4).
fn docs_from_bytes(bytes: &[u8], kind: FileKind, name: &str) -> Result<Vec<ExtractedDoc>, SkipReason> {
    let docs = match (kind, ext(Path::new(name)).as_str()) {
        (FileKind::Email, "msg") => mail::msg_docs(bytes)?,
        (FileKind::Email, "eml") => mail::eml_docs(bytes)?,
        _ => vec![ExtractedDoc::whole(kind, extract_bytes(bytes, kind, name)?)],
    };
    Ok(docs.into_iter().map(clean_doc).collect())
}

/// Files holding several documents (archives, mailboxes): each inner document
/// has its own name, so a file-name criterion is checked inside them too.
pub fn is_container(path: &Path, kind: FileKind) -> bool {
    kind == FileKind::Archive || matches!(ext(path).as_str(), "pst" | "ost" | "mbox" | "msg" | "eml")
}

/// Whether a file is read whole in memory before extraction (everything but
/// containers, which are streamed, and files too large to be documents).
pub fn preloadable(path: &Path, kind: FileKind, size: u64) -> bool {
    kind != FileKind::Archive && !matches!(ext(path).as_str(), "pst" | "ost" | "mbox") && size <= MAX_FILE_BYTES
}

/// Same as [`extract_docs`] for a file already read by the pipeline.
pub fn extract_docs_from_bytes(path: &Path, kind: FileKind, bytes: &[u8]) -> Result<Vec<ExtractedDoc>, SkipReason> {
    if !is_supported(path, kind) {
        return Err(SkipReason::Unsupported);
    }
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    docs_from_bytes(bytes, kind, name)
}

/// Plain text of one document already in memory (`name` gives the extension).
pub fn extract_bytes(bytes: &[u8], kind: FileKind, name: &str) -> Result<String, SkipReason> {
    let e = ext(Path::new(name));
    let text = match kind {
        FileKind::Email if e == "msg" => mail::msg(bytes)?,
        FileKind::Email if e == "eml" => mail::eml(bytes)?,
        FileKind::Pdf => pdf::extract(bytes)?,
        FileKind::Image => ocr::image_text(bytes)?,
        FileKind::Word if matches!(e.as_str(), "docx" | "docm") => docx::extract(bytes)?,
        FileKind::Word if e == "doc" => doc::extract(bytes)?,
        FileKind::Word if e == "rtf" => rtf::extract(bytes)?,
        FileKind::Word if e == "odt" => odf::extract(bytes)?,
        FileKind::Powerpoint if matches!(e.as_str(), "ppt") => ppt::extract(bytes)?,
        FileKind::Powerpoint if e == "odp" => odf::extract(bytes)?,
        FileKind::Text if e == "epub" => epub::extract(bytes)?,
        FileKind::Text | FileKind::Code => text::decode(bytes)?,
        FileKind::Excel if matches!(e.as_str(), "xlsx" | "xlsm" | "xls" | "xlsb" | "ods") => office::excel(bytes)?,
        FileKind::Powerpoint if matches!(e.as_str(), "pptx" | "pptm") => office::powerpoint(bytes)?,
        _ => return Err(SkipReason::Unsupported),
    };
    Ok(strip_markers(&text))
}

/// The UI marks matches with ⟦ ⟧ / ⟪ ⟫: those characters never come from files.
fn strip_markers(text: &str) -> String {
    text.replace(['⟦', '⟧', '⟪', '⟫'], "")
}

fn clean_doc(mut doc: ExtractedDoc) -> ExtractedDoc {
    doc.text = strip_markers(&doc.text);
    doc
}

/// Text of one file, or of one document inside a container (preview of a
/// file found by a live scan, which is not in the index).
pub fn extract_one(path: &Path, kind: FileKind, inner: Option<&str>) -> Result<ExtractedDoc, SkipReason> {
    let docs = extract_docs(path, kind)?;
    match inner {
        None => docs.into_iter().next().ok_or(SkipReason::Corrupt),
        Some(inner) => docs.into_iter().find(|d| d.inner.as_deref() == Some(inner)).ok_or(SkipReason::Corrupt),
    }
}

/// The bytes of documents inside one archive or e-mail (lot 6.7: "Extract
/// to…", opening it, pictures of attached images), by inner path. The
/// container is read again the way it was indexed (same inner paths, same
/// guards), but only the entries leading to them are unpacked and no text
/// is extracted. A document not found (a message, not a file; the container
/// changed) is simply missing from the map.
pub fn extract_raw(path: &Path, inners: &[&str]) -> Result<HashMap<String, Vec<u8>>, SkipReason> {
    let kind = FileKind::from_path(path).ok_or(SkipReason::Unsupported)?;
    if !is_container(path, kind) {
        return Err(SkipReason::Unsupported);
    }
    let mut error = None;
    let found = archive::capture(inners, || {
        if let Err(e) = extract_docs(path, kind) {
            error = Some(e);
        }
    });
    match error {
        // Nothing found because the container could not be read at all.
        Some(e) if found.is_empty() => Err(e),
        _ => Ok(found),
    }
}

/// The file name of a document inside a container: `y.pdf` for
/// `a.zip › x/y.pdf`.
pub fn inner_name(inner: &str) -> &str {
    let last = inner.rsplit(INNER_SEP).next().unwrap_or(inner);
    last.rsplit(['/', '\\']).next().unwrap_or(last)
}

/// Splits `C:\a.zip › x/y.pdf` into the file on disk and the inner path.
pub fn split_inner(path: &str) -> (&str, Option<&str>) {
    match path.split_once(INNER_SEP) {
        Some((file, inner)) => (file, Some(inner)),
        None => (path, None),
    }
}
