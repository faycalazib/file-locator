//! PDF text extraction (pdf-extract, pure Rust).
//!
//! Arabic (BUG-070): pdf-extract gives the text as it is drawn. Many PDFs
//! store Arabic as shaped glyphs (presentation forms, `ﻟ` for `ل`) and in
//! display order (left to right on the page, so each line reads backwards).
//! The glyphs are turned back into letters, and a document drawn backwards
//! is put back in reading order, line by line.

use unicode_normalization::UnicodeNormalization;

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
        Ok(Ok(pages)) => Ok(clean(&with_ocr(bytes, arabic(pages)).join("\n"))),
        Ok(Err(e)) => {
            let message = e.to_string().to_ascii_lowercase();
            if message.contains("encrypt") || message.contains("password") {
                // Many PDFs are "protected" only against editing or printing
                // (owner password): they open with an empty user password.
                match std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem_by_pages_encrypted(bytes, "")) {
                    Ok(Ok(pages)) => Ok(clean(&with_ocr(bytes, arabic(pages)).join("\n"))),
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

/// Arabic presentation forms: shaped glyphs (initial, medial, final,
/// isolated, ligatures), not letters.
fn is_shaped(c: char) -> bool {
    matches!(c, '\u{FB50}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFC}')
}

/// Written right to left (Arabic letters and glyphs, not digits).
fn is_rtl(c: char) -> bool {
    (matches!(c, '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}') || is_shaped(c)) && !c.is_numeric()
}

/// Written left to right: Latin words, digits (Arabic-Indic ones too).
fn is_ltr(c: char) -> bool {
    c.is_alphanumeric() && !is_rtl(c)
}

/// Arabic pages put back as letters, in reading order (BUG-070). Other
/// pages come back unchanged.
fn arabic(pages: Vec<String>) -> Vec<String> {
    if !pages.iter().any(|p| p.chars().any(is_shaped)) {
        return pages;
    }
    let (kept, turned) = article_words(&pages);
    let backwards = turned >= 3 && turned > 2 * kept;
    // Farsi yeh glyphs at the start or in the middle of a word look exactly
    // like the Arabic yeh: in an Arabic document, they are one.
    let arabic_text = kept.max(turned) >= 3;
    pages
        .into_iter()
        .map(|page| {
            let lines: Vec<String> = page
                .lines()
                .map(|line| {
                    let line = if backwards && line.chars().any(is_rtl) { reading_order(line) } else { line.to_owned() };
                    letters(&line, arabic_text)
                })
                .collect();
            lines.join("\n")
        })
        .collect()
}

/// Words starting with the article "ال", as written and turned around: an
/// Arabic text has many, a backwards one has them at the end of words.
fn article_words(pages: &[String]) -> (usize, usize) {
    let count = |text: &str| {
        text.split(|c: char| !is_rtl(c)).filter(|w| w.starts_with("ال") && w.chars().count() > 3).count()
    };
    let (mut kept, mut turned) = (0, 0);
    for line in pages.iter().flat_map(|p| p.lines()).filter(|l| l.chars().any(is_rtl)) {
        kept += count(&letters(line, false));
        turned += count(&letters(&line.chars().rev().collect::<String>(), false));
    }
    (kept, turned)
}

/// A line drawn left to right put back in reading order: turned around,
/// then Latin words and numbers read left to right again; brackets outside
/// them are mirrored.
fn reading_order(line: &str) -> String {
    let mut chars: Vec<char> = line.chars().rev().collect();
    let mut latin = vec![false; chars.len()];
    let mut i = 0;
    while i < chars.len() {
        if !is_ltr(chars[i]) {
            i += 1;
            continue;
        }
        let (start, mut end, mut j) = (i, i, i);
        while j < chars.len() && !is_rtl(chars[j]) {
            if is_ltr(chars[j]) {
                end = j;
            }
            j += 1;
        }
        chars[start..=end].reverse();
        latin[start..=end].iter_mut().for_each(|l| *l = true);
        i = end + 1;
    }
    chars
        .iter()
        .zip(&latin)
        .map(|(&c, &in_latin)| {
            if in_latin {
                return c;
            }
            match c {
                '(' => ')',
                ')' => '(',
                '[' => ']',
                ']' => '[',
                '{' => '}',
                '}' => '{',
                '<' => '>',
                '>' => '<',
                '«' => '»',
                '»' => '«',
                _ => c,
            }
        })
        .collect()
}

/// Shaped glyphs back to letters (NFKC, only on those: `ﻻ` → `لا`).
fn letters(text: &str, arabic_text: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            // Farsi yeh, initial and medial forms.
            '\u{FBFE}' | '\u{FBFF}' if arabic_text => out.push('ي'),
            c if is_shaped(c) => out.extend(std::iter::once(c).nfkc()),
            c => out.push(c),
        }
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A title drawn left to right with shaped glyphs (a real PDF, BUG-070),
    /// then two more lines so that the document is clearly backwards.
    #[test]
    fn arabic_drawn_backwards_is_read_in_order() {
        let page = "ﻢﯿﻤﺼﺘﻟ ChatGPT تارﺎﺼﺘﺧا :ﻲﻋﺎﻨﻄﺻﻻا ءﺎﻛﺬﻠﻟ بأ كﻮﻣ ﺖﺒﻣوﺮﺑو ﺮﻣأ 1000\n\
                    ﺔﯿﻓاﺮﺘﺣﻻا تﺎﻧﻼﻋﻹا ﻢﯿﻤﺼﺗ\n\
                    (AI Mockup) ﺔﯾرﺎﺠﺘﻟا ﺔﻣﻼﻌﻟا";
        let text = arabic(vec![page.replace("                    ", "")]).join("\n");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "1000 أمر وبرومبت موك أب للذكاء الاصطناعي: اختصارات ChatGPT لتصميم");
        assert_eq!(lines[1], "تصميم الإعلانات الاحترافية");
        assert_eq!(lines[2], "العلامة التجارية (AI Mockup)");
    }

    #[test]
    fn arabic_in_reading_order_stays_as_it_is() {
        // Letters already in order, some as glyphs: only the glyphs change.
        let page = "ﺍﻟﺬﻛﺎﺀ ﺍﻻﺻﻄﻨﺎﻋﻲ والتصميم والإعلانات (AI) 2026".to_owned();
        assert_eq!(arabic(vec![page]), ["الذكاء الاصطناعي والتصميم والإعلانات (AI) 2026"]);
        // No glyph at all: nothing is touched.
        let plain = "الذكاء الاصطناعي".to_owned();
        assert_eq!(arabic(vec![plain.clone()]), [plain]);
        assert_eq!(arabic(vec!["Devis n° 12 (chantier)".to_owned()]), ["Devis n° 12 (chantier)"]);
    }
}
