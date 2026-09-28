//! Scanned PDFs (lot 5.2): pages that carry an image but (almost) no text are
//! read by OCR. The page images are taken from the PDF with `lopdf` and
//! handed to the Windows recognizer:
//!
//! - JPEG (`DCTDecode`) as is;
//! - fax images (`CCITTFaxDecode`, typical of scanners), wrapped in a TIFF,
//!   which Windows decodes;
//! - raw pixels (`FlateDecode` or none; gray, RGB or CMYK, 1 or 8 bits),
//!   converted to BGRA.
//!
//! JBIG2 and JPEG 2000 cannot be decoded by Windows: those pages keep their
//! (empty) text.

use std::io::Read;

use lopdf::{Dictionary, Document, Object};

use super::ocr;

/// A page with fewer letters or digits than this is treated as scanned.
pub const SCANNED_PAGE_CHARS: usize = 32;

/// OCR text of the given pages (1-based numbers, `None` = every page).
/// Returns one entry per page read, in page order.
pub fn ocr_pages(bytes: &[u8], only: Option<&[u32]>) -> Vec<(u32, String)> {
    // lopdf may panic on malformed files, like pdf-extract (BUG-027).
    std::panic::catch_unwind(|| read_pages(bytes, only)).unwrap_or_default()
}

fn read_pages(bytes: &[u8], only: Option<&[u32]>) -> Vec<(u32, String)> {
    let Ok(doc) = Document::load_mem(bytes) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (number, page_id) in doc.get_pages() {
        if only.is_some_and(|pages| !pages.contains(&number)) {
            continue;
        }
        let Ok(images) = doc.get_page_images(page_id) else { continue };
        let mut texts = Vec::new();
        for image in images {
            let (Ok(width), Ok(height)) = (u32::try_from(image.width), u32::try_from(image.height)) else { continue };
            let filters = image.filters.clone().unwrap_or_default();
            let params = decode_params(&doc, image.origin_dict);
            let text = match filters.last().map(String::as_str) {
                Some("DCTDecode") => ocr::image_text(image.content).ok(),
                Some("CCITTFaxDecode") => {
                    let tiff = ccitt_tiff(image.content, width, height, params.as_ref());
                    ocr::image_text(&tiff).ok()
                }
                Some("FlateDecode") | None => {
                    let raw = if filters.is_empty() { Some(image.content.to_vec()) } else { inflate(image.content) };
                    raw.and_then(|raw| {
                        let raw = unpredict(raw, params.as_ref(), &image, width)?;
                        let bgra = to_bgra(&raw, &image, width, height)?;
                        ocr::pixels_text(&bgra, width, height).ok()
                    })
                }
                _ => None,
            };
            if let Some(text) = text.filter(|t| !t.trim().is_empty()) {
                texts.push(text);
            }
        }
        if !texts.is_empty() {
            out.push((number, texts.join("\n")));
        }
    }
    out
}

fn decode_params(doc: &Document, dict: &Dictionary) -> Option<Dictionary> {
    let value = dict.get(b"DecodeParms").ok()?;
    let value = match value {
        Object::Reference(id) => doc.get_object(*id).ok()?,
        other => other,
    };
    match value {
        Object::Dictionary(d) => Some(d.clone()),
        // One entry per filter: the last filter's parameters.
        Object::Array(list) => list.last().and_then(|o| o.as_dict().ok()).cloned(),
        _ => None,
    }
}

fn int(params: Option<&Dictionary>, key: &[u8], default: i64) -> i64 {
    params.and_then(|p| p.get(key).ok()).and_then(|o| o.as_i64().ok()).unwrap_or(default)
}

fn inflate(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(data).read_to_end(&mut out).ok()?;
    Some(out)
}

fn components(image: &lopdf::xobject::PdfImage) -> Option<usize> {
    match image.color_space.as_deref() {
        Some("DeviceGray" | "CalGray") => Some(1),
        Some("DeviceRGB" | "CalRGB") => Some(3),
        Some("DeviceCMYK") => Some(4),
        // ICC-based spaces: guess from the data size later; indexed: skip.
        Some("ICCBased") => Some(0),
        _ => None,
    }
}

/// PNG predictors (Predictor ≥ 10), used by many PDF writers on Flate data.
fn unpredict(raw: Vec<u8>, params: Option<&Dictionary>, image: &lopdf::xobject::PdfImage, width: u32) -> Option<Vec<u8>> {
    if int(params, b"Predictor", 1) < 10 {
        return Some(raw);
    }
    let colors = int(params, b"Colors", components(image).filter(|&c| c > 0).unwrap_or(1) as i64) as usize;
    let bits = int(params, b"BitsPerComponent", image.bits_per_component.unwrap_or(8)) as usize;
    let columns = int(params, b"Columns", i64::from(width)) as usize;
    let bpp = (colors * bits).div_ceil(8).max(1);
    let row = (colors * bits * columns).div_ceil(8);
    let mut out = Vec::with_capacity(raw.len());
    let mut previous = vec![0u8; row];
    for chunk in raw.chunks(row + 1) {
        if chunk.len() < row + 1 {
            break;
        }
        let (kind, data) = (chunk[0], &chunk[1..]);
        let mut current = data.to_vec();
        for i in 0..row {
            let left = if i >= bpp { current[i - bpp] } else { 0 };
            let up = previous[i];
            let up_left = if i >= bpp { previous[i - bpp] } else { 0 };
            current[i] = match kind {
                1 => current[i].wrapping_add(left),
                2 => current[i].wrapping_add(up),
                3 => current[i].wrapping_add(((u16::from(left) + u16::from(up)) / 2) as u8),
                4 => current[i].wrapping_add(paeth(left, up, up_left)),
                _ => current[i],
            };
        }
        out.extend_from_slice(&current);
        previous = current;
    }
    Some(out)
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = i16::from(a) + i16::from(b) - i16::from(c);
    let (pa, pb, pc) = ((p - i16::from(a)).abs(), (p - i16::from(b)).abs(), (p - i16::from(c)).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Gray / RGB / CMYK samples (1 or 8 bits) → BGRA, opaque.
fn to_bgra(raw: &[u8], image: &lopdf::xobject::PdfImage, width: u32, height: u32) -> Option<Vec<u8>> {
    let (w, h) = (width as usize, height as usize);
    let bits = image.bits_per_component.unwrap_or(8);
    let mut comps = components(image)?;
    if comps == 0 {
        // ICCBased: deduce the number of components from the data size.
        comps = raw.len() / (w * h).max(1);
        if !matches!(comps, 1 | 3 | 4) {
            return None;
        }
    }
    let mut out = Vec::with_capacity(w * h * 4);
    match (bits, comps) {
        (8, n) => {
            if raw.len() < w * h * n {
                return None;
            }
            for px in raw.chunks_exact(n).take(w * h) {
                let (r, g, b) = match n {
                    1 => (px[0], px[0], px[0]),
                    3 => (px[0], px[1], px[2]),
                    _ => {
                        let k = 255 - u16::from(px[3]);
                        let ch = |c: u8| ((255 - u16::from(c)) * k / 255) as u8;
                        (ch(px[0]), ch(px[1]), ch(px[2]))
                    }
                };
                out.extend_from_slice(&[b, g, r, 255]);
            }
        }
        (1, 1) => {
            // One bit per pixel, rows padded to a byte; 0 = black.
            let row = w.div_ceil(8);
            if raw.len() < row * h {
                return None;
            }
            for y in 0..h {
                for x in 0..w {
                    let bit = (raw[y * row + x / 8] >> (7 - x % 8)) & 1;
                    let v = if bit == 1 { 255 } else { 0 };
                    out.extend_from_slice(&[v, v, v, 255]);
                }
            }
        }
        _ => return None,
    }
    Some(out)
}

/// A fax-coded PDF image wrapped in a one-strip TIFF that Windows can decode.
fn ccitt_tiff(data: &[u8], width: u32, height: u32, params: Option<&Dictionary>) -> Vec<u8> {
    let k = int(params, b"K", 0);
    let columns = int(params, b"Columns", 1728).max(1) as u32;
    let rows = int(params, b"Rows", i64::from(height)) as u32;
    let rows = if rows == 0 { height } else { rows };
    let black_is_1 = params.and_then(|p| p.get(b"BlackIs1").ok()).and_then(|o| o.as_bool().ok()).unwrap_or(false);
    let width = if columns > 0 { columns } else { width };
    // K < 0: pure 2D (Group 4); K = 0: 1D (Group 3); K > 0: mixed (Group 3, 2D).
    let (compression, options_tag, options) = match k {
        k if k < 0 => (4u16, 293u16, 0u32),
        0 => (3, 292, 0),
        _ => (3, 292, 1),
    };
    // PDF default: 0 bits are black → TIFF "BlackIsZero".
    let photometric: u16 = if black_is_1 { 0 } else { 1 };
    let entries: Vec<(u16, u16, u32)> = vec![
        (256, 4, width),
        (257, 4, rows),
        (258, 3, 1),
        (259, 3, u32::from(compression)),
        (262, 3, u32::from(photometric)),
        (273, 4, 0), // strip offset, patched below
        (277, 3, 1),
        (278, 4, rows),
        (279, 4, data.len() as u32),
        (options_tag, 4, options),
    ];
    let ifd_size = 2 + entries.len() * 12 + 4;
    let data_offset = (8 + ifd_size) as u32;
    let mut out = Vec::with_capacity(data_offset as usize + data.len());
    out.extend_from_slice(b"II*\0");
    out.extend_from_slice(&8u32.to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    for (tag, kind, value) in entries {
        let value = if tag == 273 { data_offset } else { value };
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        if kind == 3 {
            out.extend_from_slice(&(value as u16).to_le_bytes());
            out.extend_from_slice(&[0, 0]);
        } else {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(data);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_predictors_are_undone() {
        // Two rows of 3 gray pixels, "Up" then "Sub" filters.
        let raw = vec![2, 10, 20, 30, 1, 1, 1, 1];
        let dict = Dictionary::from_iter(vec![("Predictor", Object::Integer(12)), ("Columns", Object::Integer(3))]);
        let image_dict = Dictionary::new();
        let image = lopdf::xobject::PdfImage {
            id: (1, 0),
            width: 3,
            height: 2,
            color_space: Some("DeviceGray".into()),
            filters: None,
            bits_per_component: Some(8),
            content: &[],
            origin_dict: &image_dict,
        };
        let out = unpredict(raw, Some(&dict), &image, 3).unwrap();
        assert_eq!(out, [10, 20, 30, 1, 2, 3]);
    }

    #[test]
    fn a_fax_image_gets_a_valid_tiff_header() {
        let tiff = ccitt_tiff(&[0xAA; 10], 100, 50, None);
        assert_eq!(&tiff[..4], b"II*\0");
        assert_eq!(u16::from_le_bytes([tiff[8], tiff[9]]), 10, "10 tags");
        assert_eq!(&tiff[tiff.len() - 10..], &[0xAA; 10]);
    }
}
