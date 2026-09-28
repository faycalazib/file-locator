//! OCR (lot 5.2): the text of images and scanned PDFs, read with the
//! recognizer built into Windows 10/11 (`Windows.Media.Ocr`). Nothing to
//! install: it reads the languages installed in Windows (French, English,
//! Arabic…). On other systems the functions report `Unsupported`.
//!
//! Latin and Arabic scripts need different recognizers: when both are
//! available, an image is read by both and the reading with more letters of
//! its own script wins.
//!
//! One recognizer per thread: a Windows `OcrEngine` does not take several
//! recognitions at once (concurrent calls fail silently), and the indexing
//! pipeline reads files on all cores.

use std::sync::atomic::{AtomicBool, Ordering};

use super::SkipReason;

/// Settings → "Read the text of images and scanned PDFs" (on by default).
static ENABLED: AtomicBool = AtomicBool::new(true);

/// Images smaller than this on either side are icons or thumbnails: skipped.
pub const MIN_SIDE: u32 = 200;

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

/// OCR is switched on and possible on this system.
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed) && imp::available()
}

/// Languages the recognizer can read (BCP 47 tags: `fr-FR`, `ar-SA`…).
pub fn languages() -> Vec<String> {
    imp::languages()
}

/// Text of an encoded image (PNG, JPEG, TIFF, BMP, GIF, WebP…).
pub fn image_text(bytes: &[u8]) -> Result<String, SkipReason> {
    if !enabled() {
        return Err(SkipReason::Unsupported);
    }
    imp::encoded(bytes)
}

/// Text of raw BGRA pixels (images decoded from a PDF).
pub fn pixels_text(bgra: &[u8], width: u32, height: u32) -> Result<String, SkipReason> {
    if !enabled() {
        return Err(SkipReason::Unsupported);
    }
    imp::pixels(bgra, width, height)
}

/// A word read on an image, with its box as fractions of the image (0 to 1),
/// the image being turned like the photo (EXIF orientation).
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct OcrWord {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The lines of words read on an image (lot 6.6: boxes in the preview).
/// Unlike [`image_text`], works whatever the OCR setting: the user asked
/// to see this image.
pub fn image_words(bytes: &[u8]) -> Result<Vec<Vec<OcrWord>>, SkipReason> {
    imp::words(bytes)
}

/// Letters of each script in a reading: (Latin, Arabic).
fn script_letters(text: &str) -> (usize, usize) {
    text.chars().fold((0, 0), |(latin, arabic), c| match c {
        '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{FB50}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}' => (latin, arabic + 1),
        c if c.is_alphabetic() => (latin + 1, arabic),
        _ => (latin, arabic),
    })
}

/// Picks the better of a Latin and an Arabic reading of the same image.
fn best_reading(latin: Option<String>, arabic: Option<String>) -> String {
    match (latin, arabic) {
        (Some(l), Some(a)) => {
            if script_letters(&a).1 > script_letters(&l).0 {
                a
            } else {
                l
            }
        }
        (Some(one), None) | (None, Some(one)) => one,
        (None, None) => String::new(),
    }
}

#[cfg(windows)]
mod imp {
    use std::cell::OnceCell;
    use std::sync::OnceLock;

    use windows::core::HSTRING;
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::{
        BitmapAlphaMode, BitmapDecoder, BitmapPixelFormat, BitmapTransform, ColorManagementMode, ExifOrientationMode, SoftwareBitmap,
    };
    use windows::Media::Ocr::OcrResult;
    use windows::Media::Ocr::OcrEngine;
    use windows::Security::Cryptography::CryptographicBuffer;
    use windows::Storage::Streams::InMemoryRandomAccessStream;

    use super::{best_reading, script_letters, OcrWord, SkipReason, MIN_SIDE};

    struct Engines {
        latin: Option<OcrEngine>,
        arabic: Option<OcrEngine>,
    }

    /// Recognizer languages installed in Windows (read once).
    fn tags() -> &'static [String] {
        static TAGS: OnceLock<Vec<String>> = OnceLock::new();
        TAGS.get_or_init(|| {
            OcrEngine::AvailableRecognizerLanguages()
                .map(|list| list.into_iter().filter_map(|l| l.LanguageTag().ok()).map(|t| t.to_string()).collect())
                .unwrap_or_default()
        })
    }

    fn create_engines() -> Engines {
        let create =
            |tag: &str| Language::CreateLanguage(&HSTRING::from(tag)).and_then(|l| OcrEngine::TryCreateFromLanguage(&l)).ok();
        // Latin script: the user's own language first when it is one.
        let profile = OcrEngine::TryCreateFromUserProfileLanguages()
            .ok()
            .filter(|e| e.RecognizerLanguage().and_then(|l| l.LanguageTag()).is_ok_and(|t| !t.to_string().starts_with("ar")));
        let latin = profile
            .or_else(|| tags().iter().find(|t| ["fr", "en", "es"].iter().any(|p| t.starts_with(p))).and_then(|t| create(t)));
        let arabic = tags().iter().find(|t| t.starts_with("ar")).and_then(|t| create(t));
        Engines { latin, arabic }
    }

    thread_local! {
        static ENGINES: OnceCell<Engines> = const { OnceCell::new() };
    }

    pub fn available() -> bool {
        tags().iter().any(|t| ["fr", "en", "es", "ar"].iter().any(|p| t.starts_with(p)))
    }

    pub fn languages() -> Vec<String> {
        tags().to_vec()
    }

    fn read(bitmap: &SoftwareBitmap) -> Result<String, SkipReason> {
        let (w, h) = (bitmap.PixelWidth().unwrap_or(0), bitmap.PixelHeight().unwrap_or(0));
        if w.min(h) < MIN_SIDE as i32 {
            return Err(SkipReason::Unsupported);
        }
        let max = OcrEngine::MaxImageDimension().unwrap_or(10_000) as i32;
        if w.max(h) > max {
            return Err(SkipReason::TooLarge);
        }
        // The recognizer wants 8-bit BGRA or gray pixels.
        let bitmap = SoftwareBitmap::Convert(bitmap, BitmapPixelFormat::Bgra8).map_err(|_| SkipReason::Corrupt)?;
        let run = |engine: &Option<OcrEngine>| -> Option<String> {
            let result = engine.as_ref()?.RecognizeAsync(&bitmap).ok()?.join().ok()?;
            let lines: Vec<String> = result.Lines().ok()?.into_iter().filter_map(|l| l.Text().ok()).map(|t| t.to_string()).collect();
            Some(lines.join("\n"))
        };
        Ok(ENGINES.with(|cell| {
            let e = cell.get_or_init(create_engines);
            best_reading(run(&e.latin), run(&e.arabic))
        }))
    }

    pub fn encoded(bytes: &[u8]) -> Result<String, SkipReason> {
        let decode = || -> windows::core::Result<SoftwareBitmap> {
            let buffer = CryptographicBuffer::CreateFromByteArray(bytes)?;
            let stream = InMemoryRandomAccessStream::new()?;
            stream.WriteAsync(&buffer)?.join()?;
            stream.Seek(0)?;
            let decoder = BitmapDecoder::CreateAsync(&stream)?.join()?;
            decoder.GetSoftwareBitmapAsync()?.join()
        };
        read(&decode().map_err(|_| SkipReason::Corrupt)?)
    }

    /// Words with their boxes, on the image turned like the photo.
    pub fn words(bytes: &[u8]) -> Result<Vec<Vec<OcrWord>>, SkipReason> {
        let decode = || -> windows::core::Result<SoftwareBitmap> {
            let stream = InMemoryRandomAccessStream::new()?;
            stream.WriteAsync(&CryptographicBuffer::CreateFromByteArray(bytes)?)?.join()?;
            stream.Seek(0)?;
            let decoder = BitmapDecoder::CreateAsync(&stream)?.join()?;
            decoder
                .GetSoftwareBitmapTransformedAsync(
                    BitmapPixelFormat::Bgra8,
                    BitmapAlphaMode::Premultiplied,
                    &BitmapTransform::new()?,
                    ExifOrientationMode::RespectExifOrientation,
                    ColorManagementMode::ColorManageToSRgb,
                )?
                .join()
        };
        let bitmap = decode().map_err(|_| SkipReason::Corrupt)?;
        let (w, h) = (bitmap.PixelWidth().unwrap_or(0), bitmap.PixelHeight().unwrap_or(0));
        let max = OcrEngine::MaxImageDimension().unwrap_or(10_000) as i32;
        if w <= 0 || h <= 0 || w.max(h) > max {
            return Err(SkipReason::TooLarge);
        }
        let run = |engine: &Option<OcrEngine>| -> Option<OcrResult> { engine.as_ref()?.RecognizeAsync(&bitmap).ok()?.join().ok() };
        let text = |result: &OcrResult| result.Text().map(|t| t.to_string()).unwrap_or_default();
        let result = ENGINES.with(|cell| {
            let e = cell.get_or_init(create_engines);
            match (run(&e.latin), run(&e.arabic)) {
                // Same rule as the indexed reading: more letters of its own script.
                (Some(l), Some(a)) => Some(if script_letters(&text(&a)).1 > script_letters(&text(&l)).0 { a } else { l }),
                (Some(one), None) | (None, Some(one)) => Some(one),
                (None, None) => None,
            }
        });
        let Some(result) = result else { return Ok(Vec::new()) };
        let (fw, fh) = (w as f32, h as f32);
        let lines = result
            .Lines()
            .map(|lines| {
                lines
                    .into_iter()
                    .filter_map(|line| line.Words().ok())
                    .map(|words| {
                        words
                            .into_iter()
                            .filter_map(|word| {
                                let r = word.BoundingRect().ok()?;
                                Some(OcrWord { text: word.Text().ok()?.to_string(), x: r.X / fw, y: r.Y / fh, w: r.Width / fw, h: r.Height / fh })
                            })
                            .collect()
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(lines)
    }

    pub fn pixels(bgra: &[u8], width: u32, height: u32) -> Result<String, SkipReason> {
        let bitmap = CryptographicBuffer::CreateFromByteArray(bgra)
            .and_then(|b| SoftwareBitmap::CreateCopyFromBuffer(&b, BitmapPixelFormat::Bgra8, width as i32, height as i32))
            .map_err(|_| SkipReason::Corrupt)?;
        read(&bitmap)
    }
}

#[cfg(not(windows))]
mod imp {
    use super::SkipReason;

    pub fn available() -> bool {
        false
    }

    pub fn languages() -> Vec<String> {
        Vec::new()
    }

    pub fn encoded(_: &[u8]) -> Result<String, SkipReason> {
        Err(SkipReason::Unsupported)
    }

    pub fn pixels(_: &[u8], _: u32, _: u32) -> Result<String, SkipReason> {
        Err(SkipReason::Unsupported)
    }

    pub fn words(_: &[u8]) -> Result<Vec<Vec<super::OcrWord>>, SkipReason> {
        Err(SkipReason::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reading_with_more_letters_of_its_script_wins() {
        let latin = Some("Facture numero 4271".to_owned());
        let garbage_arabic = Some("ل ٤".to_owned());
        assert_eq!(best_reading(latin.clone(), garbage_arabic), "Facture numero 4271");
        let arabic = Some("عقد إيجار محل تجاري".to_owned());
        let garbage_latin = Some("sl J".to_owned());
        assert_eq!(best_reading(garbage_latin, arabic), "عقد إيجار محل تجاري");
        assert_eq!(best_reading(None, None), "");
    }
}
