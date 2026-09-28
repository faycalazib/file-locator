//! Thumbnails of images (lot 6.6), with the imaging built into Windows (the
//! one the OCR uses): no library to ship. The camera's orientation (EXIF) is
//! respected; the result is a JPEG.

use std::path::Path;

use crate::error::{CoreError, Result};

/// Images larger than this are not read for a thumbnail.
const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// A JPEG of `path`, its longest side at most `max_side` pixels (never
/// enlarged), with its width and height.
pub fn thumbnail(path: &Path, max_side: u32) -> Result<(Vec<u8>, u32, u32)> {
    let unreadable = || CoreError::Storage { message: format!("cannot read image {}", path.display()) };
    if std::fs::metadata(path).map_err(|_| unreadable())?.len() > MAX_BYTES {
        return Err(unreadable());
    }
    let bytes = std::fs::read(path).map_err(|_| unreadable())?;
    imp::thumbnail(&bytes, max_side.max(16)).ok_or_else(unreadable)
}

/// Same as [`thumbnail`] for an image already in memory (an image inside an
/// archive or attached to an e-mail, lot 6.7).
pub fn thumbnail_of_bytes(bytes: &[u8], max_side: u32) -> Result<(Vec<u8>, u32, u32)> {
    imp::thumbnail(bytes, max_side.max(16)).ok_or_else(|| CoreError::Storage { message: "cannot read image".to_owned() })
}

#[cfg(windows)]
mod imp {
    use windows::Graphics::Imaging::{
        BitmapAlphaMode, BitmapDecoder, BitmapEncoder, BitmapInterpolationMode, BitmapPixelFormat, BitmapTransform, ColorManagementMode,
        ExifOrientationMode,
    };
    use windows::Security::Cryptography::CryptographicBuffer;
    use windows::Storage::Streams::{DataReader, InMemoryRandomAccessStream};

    pub fn thumbnail(bytes: &[u8], max_side: u32) -> Option<(Vec<u8>, u32, u32)> {
        let run = || -> windows::core::Result<(Vec<u8>, u32, u32)> {
            let input = InMemoryRandomAccessStream::new()?;
            input.WriteAsync(&CryptographicBuffer::CreateFromByteArray(bytes)?)?.join()?;
            input.Seek(0)?;
            let decoder = BitmapDecoder::CreateAsync(&input)?.join()?;
            // The scale applies before the orientation (Windows order: scale,
            // flip, rotation): computed on the stored frame, then the output
            // is turned like the photo.
            let (w, h) = (decoder.PixelWidth()?, decoder.PixelHeight()?);
            let scale = (f64::from(max_side) / f64::from(w.max(h))).min(1.0);
            let (sw, sh) = (((f64::from(w) * scale).round() as u32).max(1), ((f64::from(h) * scale).round() as u32).max(1));
            let turned = decoder.OrientedPixelWidth()? != w;
            let (ow, oh) = if turned { (sh, sw) } else { (sw, sh) };
            let transform = BitmapTransform::new()?;
            transform.SetScaledWidth(sw)?;
            transform.SetScaledHeight(sh)?;
            transform.SetInterpolationMode(BitmapInterpolationMode::Fant)?;
            let pixels = decoder
                .GetPixelDataTransformedAsync(
                    BitmapPixelFormat::Bgra8,
                    BitmapAlphaMode::Ignore,
                    &transform,
                    ExifOrientationMode::RespectExifOrientation,
                    ColorManagementMode::ColorManageToSRgb,
                )?
                .join()?
                .DetachPixelData()?;
            let output = InMemoryRandomAccessStream::new()?;
            let encoder = BitmapEncoder::CreateAsync(BitmapEncoder::JpegEncoderId()?, &output)?.join()?;
            encoder.SetPixelData(BitmapPixelFormat::Bgra8, BitmapAlphaMode::Ignore, ow, oh, 96.0, 96.0, &pixels)?;
            encoder.FlushAsync()?.join()?;
            let size = output.Size()? as u32;
            let reader = DataReader::CreateDataReader(&output.GetInputStreamAt(0)?)?;
            reader.LoadAsync(size)?.join()?;
            let mut jpeg = vec![0u8; size as usize];
            reader.ReadBytes(&mut jpeg)?;
            Ok((jpeg, ow, oh))
        };
        run().ok()
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn thumbnail(_: &[u8], _: u32) -> Option<(Vec<u8>, u32, u32)> {
        None
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn a_thumbnail_is_a_smaller_jpeg() {
        let image = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("test_fixtures").join("images");
        let first = std::fs::read_dir(&image).unwrap().flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|e| e == "png")).unwrap();
        let (jpeg, w, h) = thumbnail(&first, 64).unwrap();
        assert!(w.max(h) <= 64 && w.min(h) >= 1, "{w}×{h}");
        assert_eq!(&jpeg[..2], &[0xFF, 0xD8], "JPEG start of image");
        // Never enlarged.
        let (_, w, h) = thumbnail(&first, 100_000).unwrap();
        assert!(w.max(h) < 100_000);
        assert!(thumbnail(Path::new("absent.png"), 64).is_err());
    }
}
