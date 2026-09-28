//! PDF text extraction (pdf-extract, pure Rust).

use super::pdf_ocr::{ocr_pages, SCANNED_PAGE_CHARS};
use super::{ocr, SkipReason};

// catch_unwind below does nothing when panics abort: a single damaged PDF
// would close the whole app (BUG-027). Keep `panic = "unwind"`.
#[cfg(panic = "abort")]
compile_error!("prospector-core must be built with panic = \"unwind\" (PDF extraction catches panics, BUG-027)");

pub fn extract(bytes: &[u8]) -> Result<String, SkipReason> {
    // pdf-extract may panic on malformed files: one bad PDF must never stop
    // the indexing of a whole folder.
    let result = std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem_by_pages(bytes));
    match result {
        Ok(Ok(pages)) => Ok(clean(&with_ocr(bytes, pages).join("\n"))),
        Ok(Err(e)) => {
            let message = e.to_string().to_ascii_lowercase();
            if message.contains("encrypt") || message.contains("password") {
                // Many PDFs are "protected" only against editing or printing
                // (owner password): they open with an empty user password.
                match std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem_by_pages_encrypted(bytes, "")) {
                    Ok(Ok(pages)) => Ok(clean(&with_ocr(bytes, pages).join("\n"))),
                    _ => Err(SkipReason::Encrypted),
                }
            } else {
                // Some scans have no readable text layer at all: OCR them whole.
                scanned_only(bytes).ok_or(SkipReason::Corrupt)
            }
        }
        Err(_) => scanned_only(bytes).ok_or(SkipReason::Corrupt),
    }
}

/// Pages with (almost) no text are read by OCR (scanned pages, lot 5.2).
fn with_ocr(bytes: &[u8], mut pages: Vec<String>) -> Vec<String> {
    if !ocr::enabled() {
        return pages;
    }
    let scanned: Vec<u32> = pages
        .iter()
        .enumerate()
        .filter(|(_, text)| text.chars().filter(|c| c.is_alphanumeric()).count() < SCANNED_PAGE_CHARS)
        .map(|(i, _)| i as u32 + 1)
        .collect();
    if scanned.is_empty() {
        return pages;
    }
    for (number, text) in ocr_pages(bytes, Some(&scanned)) {
        if let Some(page) = pages.get_mut(number as usize - 1) {
            *page = text;
        }
    }
    pages
}

/// A PDF whose text layer cannot be read: its page images, by OCR.
fn scanned_only(bytes: &[u8]) -> Option<String> {
    if !ocr::enabled() {
        return None;
    }
    let pages = ocr_pages(bytes, None);
    (!pages.is_empty()).then(|| clean(&pages.into_iter().map(|(_, t)| t).collect::<Vec<_>>().join("\n")))
}

/// pdf-extract emits page breaks as form feeds and many blank lines.
fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank_run = 0;
    for line in text.replace('\u{000C}', "\n").lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_owned()
}
