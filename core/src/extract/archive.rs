//! Archives: ZIP (and JAR), 7z, RAR, TAR (plain, gzip or bzip2 compressed),
//! and single files compressed with gzip or bzip2 (`access.log.gz`).
//! Every readable entry becomes its own document
//! (`archive.zip › folder/contract.pdf`). Nested ZIP and 7z archives are read
//! up to `MAX_DEPTH` (a RAR can only be opened from a file on disk: a RAR
//! inside another archive is skipped). Guards against archive bombs:
//! per-entry size, total budget, compression ratio (ZIP) and entry count.
//!
//! - ZIP: `zip` crate.
//! - 7z: `sevenz-rust2` (pure Rust). In a solid archive the entries are one
//!   compressed stream, so a skipped entry is still read through.
//! - RAR (4 and 5): `unrar`, the official UnRAR library (free licence,
//!   extraction only). Multi-volume sets are read from their first part.
//!
//! Lot 6.7: the same walk also gives back the bytes of chosen documents
//! ([`capture`]): only the entries leading to them are unpacked, and no text
//! is extracted, so their inner paths are exactly the indexed ones.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Seek};
use std::path::Path;
use std::time::UNIX_EPOCH;

use sevenz_rust2::{ArchiveReader, Password};

use super::{extract_bytes, ExtractedDoc, SkipReason, INNER_SEP};
use crate::kind::FileKind;

const MAX_DEPTH: usize = 3;
const MAX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 20_000;
/// A 100× compression ratio on a big entry is a zip bomb, not a document.
const MAX_RATIO: u64 = 100;

/// What has been read so far in one archive (nested ones included).
struct Budget {
    bytes: u64,
    entries: usize,
    /// Entries skipped because they are password-protected.
    encrypted: usize,
    /// Family of the documents found inside: `Archive` for an archive,
    /// `Email` for the attachments of a message.
    family: FileKind,
}

impl Budget {
    fn spent(&self) -> bool {
        self.entries >= MAX_ENTRIES || self.bytes >= MAX_TOTAL_BYTES || capture_done()
    }
}

/// Documents wanted by a running [`capture`], and those met so far.
struct Capture {
    wants: Vec<String>,
    found: HashMap<String, Vec<u8>>,
}

thread_local! {
    /// Set only while [`capture`] runs on this thread (indexing never sets it).
    static CAPTURE: RefCell<Option<Capture>> = const { RefCell::new(None) };
}

/// Runs `walk` (the usual reading of a container) and returns the bytes of
/// the documents `wants` met on the way, by inner path.
pub(super) fn capture(wants: &[&str], walk: impl FnOnce()) -> HashMap<String, Vec<u8>> {
    /// Clears the capture even if the walk panics.
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            CAPTURE.with_borrow_mut(|c| *c = None);
        }
    }
    let wants = wants.iter().map(|w| (*w).to_owned()).collect();
    CAPTURE.with_borrow_mut(|c| *c = Some(Capture { wants, found: HashMap::new() }));
    let _reset = Reset;
    walk();
    CAPTURE.with_borrow_mut(|c| c.take().map(|c| c.found).unwrap_or_default())
}

/// A capture is running and has everything it wants: the walk can stop.
pub(super) fn capture_done() -> bool {
    CAPTURE.with_borrow(|c| c.as_ref().is_some_and(|c| c.found.len() >= c.wants.len()))
}

/// A capture is running and `prefix › name` is neither a wanted document nor
/// on the way to one: the entry is not read at all.
pub(super) fn capture_skips(prefix: &str, name: &str) -> bool {
    CAPTURE.with_borrow(|c| {
        let Some(c) = c.as_ref() else { return false };
        let inner = if prefix.is_empty() { name.to_owned() } else { format!("{prefix}{INNER_SEP}{name}") };
        let below = format!("{inner}{INNER_SEP}");
        !c.wants.iter().any(|w| !c.found.contains_key(w) && (*w == inner || w.starts_with(&below)))
    })
}

/// While capturing: keeps the bytes of a wanted document (`None`: nothing
/// more to do with them). `Some`: go on as usual (indexing, or an archive
/// on the way to a wanted document).
fn offer(inner: &str, kind: FileKind, bytes: Vec<u8>) -> Option<Vec<u8>> {
    CAPTURE.with_borrow_mut(|c| match c.as_mut() {
        None => Some(bytes),
        Some(c) if c.wants.iter().any(|w| w == inner) => {
            c.found.insert(inner.to_owned(), bytes);
            None
        }
        Some(_) => (kind == FileKind::Archive).then_some(bytes),
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Zip,
    SevenZ,
    Rar,
    Tar,
    TarGz,
    TarBz2,
    /// One compressed file: its name without `.gz`.
    Gz,
    Bz2,
}

fn format_of(name: &str) -> Option<Format> {
    let lower = name.to_ascii_lowercase();
    let ends = |suffixes: &[&str]| suffixes.iter().any(|s| lower.ends_with(s));
    Some(if ends(&[".zip", ".jar"]) {
        Format::Zip
    } else if ends(&[".7z"]) {
        Format::SevenZ
    } else if ends(&[".rar"]) {
        Format::Rar
    } else if ends(&[".tar.gz", ".tgz"]) {
        Format::TarGz
    } else if ends(&[".tar.bz2", ".tbz2", ".tbz"]) {
        Format::TarBz2
    } else if ends(&[".tar"]) {
        Format::Tar
    } else if ends(&[".gz"]) {
        Format::Gz
    } else if ends(&[".bz2"]) {
        Format::Bz2
    } else {
        return None;
    })
}

/// Archive formats that can be read (by extension).
pub fn is_supported(path: &Path) -> bool {
    format_of(&path.to_string_lossy()).is_some()
}

/// Every document of an archive on disk.
pub fn archive_file(path: &Path) -> Result<Vec<ExtractedDoc>, SkipReason> {
    let format = format_of(&path.to_string_lossy()).ok_or(SkipReason::Unsupported)?;
    let mut budget = Budget { bytes: 0, entries: 0, encrypted: 0, family: FileKind::Archive };
    let mut out = Vec::new();
    match format {
        Format::Rar => read_rar(path, &mut budget, &mut out)?,
        _ => {
            let file = std::fs::File::open(path).map_err(|_| SkipReason::Unreadable)?;
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            read_stream(file, format, &name, "", 0, &mut budget, &mut out)?;
        }
    }
    // Nothing readable because everything is password-protected.
    if out.is_empty() && budget.encrypted > 0 {
        return Err(SkipReason::Encrypted);
    }
    Ok(out)
}

/// An e-mail attachment (lot 5.4) → its documents: the file itself, or the
/// entries of an attached archive, with the same limits as archive entries.
/// `prefix`: where the message sits (`""` for a lone .msg, a PST folder path…).
pub(super) fn attachment_docs(name: &str, bytes: Vec<u8>, modified: Option<u64>, prefix: &str, out: &mut Vec<ExtractedDoc>) {
    let mut budget = Budget { bytes: 0, entries: 0, encrypted: 0, family: FileKind::Email };
    let size = bytes.len() as u64;
    if let Some(kind) = admit(prefix, name, size, None, 0, &mut budget) {
        push_entry(bytes, name, kind, size, modified, prefix, 0, &mut budget, out);
    }
}

/// A ZIP or 7z archive from any seekable source (file, or bytes of a nested one).
/// `name`: the archive's own name (a single `.gz` file is named after it).
#[allow(clippy::too_many_arguments)]
fn read_stream<R: Read + Seek>(
    mut reader: R,
    format: Format,
    name: &str,
    prefix: &str,
    depth: usize,
    budget: &mut Budget,
    out: &mut Vec<ExtractedDoc>,
) -> Result<(), SkipReason> {
    match format {
        Format::Zip => read_zip(reader, prefix, depth, budget, out),
        Format::SevenZ => read_7z(reader, prefix, depth, budget, out),
        Format::Rar => Err(SkipReason::Unsupported),
        Format::Tar => read_tar(reader, prefix, depth, budget, out),
        Format::TarGz => read_tar(flate2::read::GzDecoder::new(reader), prefix, depth, budget, out),
        Format::TarBz2 => read_tar(bzip2::read::BzDecoder::new(reader), prefix, depth, budget, out),
        Format::Gz | Format::Bz2 => {
            let compressed = reader.seek(std::io::SeekFrom::End(0)).map_err(|_| SkipReason::Unreadable)?;
            reader.seek(std::io::SeekFrom::Start(0)).map_err(|_| SkipReason::Unreadable)?;
            let decoder: Box<dyn Read> = if format == Format::Gz {
                Box::new(flate2::read::GzDecoder::new(reader))
            } else {
                Box::new(bzip2::read::BzDecoder::new(reader))
            };
            read_single(decoder, compressed, name, prefix, depth, budget, out)
        }
    }
}

/// TAR entries, read in sequence (a tar stream cannot seek).
fn read_tar<R: Read>(reader: R, prefix: &str, depth: usize, budget: &mut Budget, out: &mut Vec<ExtractedDoc>) -> Result<(), SkipReason> {
    let mut archive = tar::Archive::new(reader);
    let entries = archive.entries().map_err(|_| SkipReason::Corrupt)?;
    for entry in entries {
        if budget.spent() {
            break;
        }
        // A damaged stream cannot be resynchronised: stop at the first error.
        let Ok(mut entry) = entry else { break };
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let Ok(path) = entry.path() else { continue };
        let name = path.to_string_lossy().replace('\\', "/");
        let size = entry.size();
        let Some(kind) = admit(prefix, &name, size, None, depth, budget) else { continue };
        let mut bytes = Vec::with_capacity(usize::try_from(size.min(MAX_ENTRY_BYTES)).unwrap_or(0));
        if entry.by_ref().take(MAX_ENTRY_BYTES).read_to_end(&mut bytes).is_err() {
            break;
        }
        let modified = entry.header().mtime().ok();
        push_entry(bytes, &name, kind, size, modified, prefix, depth, budget, out);
    }
    Ok(())
}

/// One gzip / bzip2 compressed file: `access.log.gz` holds `access.log`.
#[allow(clippy::too_many_arguments)]
fn read_single(
    decoder: Box<dyn Read + '_>,
    compressed: u64,
    name: &str,
    prefix: &str,
    depth: usize,
    budget: &mut Budget,
    out: &mut Vec<ExtractedDoc>,
) -> Result<(), SkipReason> {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let inner = base.rsplit_once('.').map_or(base, |(stem, _)| stem).to_owned();
    // One byte more than the limit: tells "too large" from "exactly the limit".
    let mut bytes = Vec::new();
    decoder.take(MAX_ENTRY_BYTES + 1).read_to_end(&mut bytes).map_err(|_| SkipReason::Corrupt)?;
    let size = bytes.len() as u64;
    let Some(kind) = admit(prefix, &inner, size, Some(compressed), depth, budget) else {
        return Ok(());
    };
    push_entry(bytes, &inner, kind, size, None, prefix, depth, budget, out);
    Ok(())
}

/// The family of an entry if it is worth reading, and counts it in the budget.
/// `compressed`: packed size when the format gives it per entry (ratio check).
fn admit(prefix: &str, name: &str, size: u64, compressed: Option<u64>, depth: usize, budget: &mut Budget) -> Option<FileKind> {
    let kind = FileKind::from_path(Path::new(name))?;
    if capture_skips(prefix, name) {
        return None;
    }
    if size > MAX_ENTRY_BYTES {
        return None;
    }
    if let Some(compressed) = compressed {
        if size > 1024 * 1024 && size / compressed.max(1) > MAX_RATIO {
            return None;
        }
    }
    if kind == FileKind::Archive {
        // Nested archives are opened in memory: ZIP and 7z only.
        // Everything but RAR can be opened from memory.
        let nested = format_of(name).is_some_and(|f| f != Format::Rar);
        if !nested || depth + 1 >= MAX_DEPTH {
            return None;
        }
    }
    budget.entries += 1;
    budget.bytes += size;
    Some(kind)
}

/// Turns the bytes of an admitted entry into documents.
#[allow(clippy::too_many_arguments)]
fn push_entry(
    bytes: Vec<u8>,
    name: &str,
    kind: FileKind,
    size: u64,
    modified: Option<u64>,
    prefix: &str,
    depth: usize,
    budget: &mut Budget,
    out: &mut Vec<ExtractedDoc>,
) {
    let inner = if prefix.is_empty() { name.to_owned() } else { format!("{prefix}{INNER_SEP}{name}") };
    let Some(bytes) = offer(&inner, kind, bytes) else { return };
    if kind == FileKind::Archive {
        if let Some(format) = format_of(name) {
            let _ = read_stream(std::io::Cursor::new(bytes), format, name, &inner, depth + 1, budget, out);
        }
        return;
    }
    if let Ok(text) = extract_bytes(&bytes, kind, name) {
        out.push(ExtractedDoc { inner: Some(inner), kind: budget.family, size: Some(size), modified, text });
    }
}

fn read_zip<R: Read + Seek>(
    reader: R,
    prefix: &str,
    depth: usize,
    budget: &mut Budget,
    out: &mut Vec<ExtractedDoc>,
) -> Result<(), SkipReason> {
    let mut archive = zip::ZipArchive::new(reader).map_err(|_| SkipReason::Corrupt)?;
    for i in 0..archive.len() {
        if budget.spent() {
            break;
        }
        let Ok(mut entry) = archive.by_index(i) else { continue };
        if entry.is_dir() {
            continue;
        }
        if entry.encrypted() {
            budget.encrypted += 1;
            continue;
        }
        let name = entry.name().replace('\\', "/");
        let size = entry.size();
        let Some(kind) = admit(prefix, &name, size, Some(entry.compressed_size()), depth, budget) else { continue };
        let mut bytes = Vec::with_capacity(usize::try_from(size).unwrap_or(0));
        if entry.by_ref().take(MAX_ENTRY_BYTES).read_to_end(&mut bytes).is_err() {
            continue;
        }
        let modified = entry.last_modified().and_then(|t| {
            civil_to_unix(t.year().into(), t.month().into(), t.day().into(), t.hour().into(), t.minute().into(), t.second().into())
        });
        drop(entry);
        push_entry(bytes, &name, kind, size, modified, prefix, depth, budget, out);
    }
    Ok(())
}

fn read_7z<R: Read + Seek>(
    reader: R,
    prefix: &str,
    depth: usize,
    budget: &mut Budget,
    out: &mut Vec<ExtractedDoc>,
) -> Result<(), SkipReason> {
    let mut archive = ArchiveReader::new(reader, Password::empty()).map_err(|e| seven_error(&e))?;
    let found_before = out.len();
    let result = archive.for_each_entries(|entry, data| {
        if budget.spent() {
            return Ok(false);
        }
        let name = entry.name().replace('\\', "/");
        let admitted = if entry.is_directory() || !entry.has_stream() {
            None
        } else {
            admit(prefix, &name, entry.size(), None, depth, budget)
        };
        match admitted {
            Some(kind) => {
                let mut bytes = Vec::with_capacity(usize::try_from(entry.size()).unwrap_or(0));
                data.take(MAX_ENTRY_BYTES).read_to_end(&mut bytes)?;
                std::io::copy(data, &mut std::io::sink())?;
                let modified = std::time::SystemTime::from(entry.last_modified_date())
                    .duration_since(UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_secs())
                    .filter(|_| entry.has_last_modified_date);
                push_entry(bytes, &name, kind, entry.size(), modified, prefix, depth, budget, out);
            }
            // Solid archive: the next entries follow in the same stream.
            None => budget.bytes += std::io::copy(data, &mut std::io::sink())?,
        }
        Ok(true)
    });
    match result {
        Ok(()) => Ok(()),
        // Documents read before a damaged part are kept.
        Err(_) if out.len() > found_before => Ok(()),
        Err(e) => Err(seven_error(&e)),
    }
}

fn seven_error(e: &sevenz_rust2::Error) -> SkipReason {
    use sevenz_rust2::Error;
    match e {
        Error::PasswordRequired | Error::MaybeBadPassword(_) => SkipReason::Encrypted,
        _ => SkipReason::Corrupt,
    }
}

fn read_rar(path: &Path, budget: &mut Budget, out: &mut Vec<ExtractedDoc>) -> Result<(), SkipReason> {
    let archive = unrar::Archive::new(path);
    // Parts 2, 3… of a multi-volume set: read together with the first part.
    if archive.is_multipart() && archive.first_part_option().is_some_and(|first| first != path) {
        return Err(SkipReason::Unsupported);
    }
    let mut open = archive.open_for_processing().map_err(|e| rar_error(&e))?;
    loop {
        if budget.spent() {
            break;
        }
        let header = match open.read_header() {
            Ok(Some(header)) => header,
            Ok(None) => break,
            Err(e) if out.is_empty() => return Err(rar_error(&e)),
            Err(_) => break,
        };
        let entry = header.entry();
        let name = entry.filename.to_string_lossy().replace('\\', "/");
        let size = entry.unpacked_size;
        let modified = dos_time_to_unix(entry.file_time);
        let admitted = if !entry.is_file() {
            None
        } else if entry.is_encrypted() {
            budget.encrypted += 1;
            None
        } else {
            // A RAR inside a RAR cannot be opened from memory: skipped by admit.
            admit("", &name, size, None, 0, budget)
        };
        open = match admitted {
            Some(kind) => match header.read() {
                Ok((bytes, next)) => {
                    push_entry(bytes, &name, kind, size, modified, "", 0, budget, out);
                    next
                }
                Err(_) => break,
            },
            None => match header.skip() {
                Ok(next) => next,
                Err(_) => break,
            },
        };
    }
    Ok(())
}

fn rar_error(e: &unrar::error::UnrarError) -> SkipReason {
    use unrar::error::Code;
    match e.code {
        Code::MissingPassword | Code::BadPassword => SkipReason::Encrypted,
        Code::EOpen | Code::ERead => SkipReason::Unreadable,
        _ => SkipReason::Corrupt,
    }
}

/// MS-DOS date and time (ZIP, RAR) → Unix seconds.
fn dos_time_to_unix(t: u32) -> Option<u64> {
    let (date, time) = (t >> 16, t & 0xFFFF);
    civil_to_unix(
        i64::from((date >> 9) & 0x7F) + 1980,
        i64::from((date >> 5) & 0x0F),
        i64::from(date & 0x1F),
        i64::from(time >> 11),
        i64::from((time >> 5) & 0x3F),
        i64::from((time & 0x1F) * 2),
    )
}

/// Calendar date and time (UTC) → Unix seconds (Howard Hinnant's days from civil).
fn civil_to_unix(y: i64, m: i64, d: i64, hour: i64, minute: i64, second: i64) -> Option<u64> {
    if !(1..=12).contains(&m) || d < 1 {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hour * 3600 + minute * 60 + second).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dos_dates() {
        // 2024-03-15 10:30:20 → date (44 << 9 | 3 << 5 | 15), time (10 << 11 | 30 << 5 | 10).
        let t = ((44 << 9 | 3 << 5 | 15) << 16) | (10 << 11 | 30 << 5 | 10);
        assert_eq!(dos_time_to_unix(t), Some(1_710_498_620));
        assert_eq!(dos_time_to_unix(0), None);
    }

    #[test]
    fn nested_archives_only_in_memory_formats() {
        let mut budget = Budget { bytes: 0, entries: 0, encrypted: 0, family: FileKind::Archive };
        assert_eq!(admit("", "a/b.zip", 10, None, 0, &mut budget), Some(FileKind::Archive));
        assert_eq!(admit("", "a/b.7z", 10, None, 0, &mut budget), Some(FileKind::Archive));
        assert_eq!(admit("", "a/b.rar", 10, None, 0, &mut budget), None);
        assert_eq!(admit("", "a/b.zip", 10, None, MAX_DEPTH - 1, &mut budget), None);
        assert_eq!(admit("", "bomb.txt", 50 * 1024 * 1024, Some(1024), 0, &mut budget), None);
        assert_eq!(budget.entries, 2);
    }

    #[test]
    fn a_capture_opens_only_the_way_to_what_it_wants() {
        let wanted = format!("a.zip{INNER_SEP}b/c.pdf");
        let found = capture(&[&wanted], || {
            assert!(!capture_skips("", "a.zip"), "the archive holding it");
            assert!(!capture_skips("a.zip", "b/c.pdf"), "the document itself");
            assert!(capture_skips("a.zip", "b/d.pdf"), "a neighbour");
            assert!(capture_skips("", "z.zip"), "another archive");
            assert_eq!(offer(&wanted, FileKind::Pdf, vec![1, 2]), None);
            assert!(capture_done());
            assert!(capture_skips("a.zip", "b/c.pdf"), "met once is enough");
        });
        assert_eq!(found.get(&wanted), Some(&vec![1, 2]));
        // Outside a capture, nothing is skipped nor kept.
        assert!(!capture_skips("", "z.zip"));
        assert_eq!(offer("x", FileKind::Pdf, vec![3]), Some(vec![3]));
    }
}
