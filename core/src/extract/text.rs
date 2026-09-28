//! Plain text and source code: encoding detection (BOM → UTF-8 → guess).

use encoding_rs::Encoding;

use super::SkipReason;

/// Decodes a text file whatever its encoding (UTF-8/16 with BOM, UTF-8,
/// Windows-1252, ISO-8859-x, Windows-1256 for Arabic…).
pub fn decode(bytes: &[u8]) -> Result<String, SkipReason> {
    if let Some((encoding, bom_len)) = Encoding::for_bom(bytes) {
        let (text, _) = encoding.decode_without_bom_handling(&bytes[bom_len..]);
        return Ok(text.into_owned());
    }
    if looks_binary(bytes) {
        return Err(SkipReason::Binary);
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(text.to_owned());
    }
    let mut detector = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Deny);
    detector.feed(bytes, true);
    let encoding = detector.guess(None, chardetng::Utf8Detection::Allow);
    let (text, _, _) = encoding.decode(bytes);
    Ok(text.into_owned())
}

/// NUL bytes in the first 8 KiB mean a binary file (UTF-16 has a BOM).
fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|&b| b == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf8_and_utf16_bom() {
        assert_eq!(decode("déjà vu".as_bytes()).unwrap(), "déjà vu");
        let utf16: Vec<u8> = [0xFF, 0xFE].into_iter().chain("été".encode_utf16().flat_map(u16::to_le_bytes)).collect();
        assert_eq!(decode(&utf16).unwrap(), "été");
    }

    #[test]
    fn decodes_windows_1252() {
        // "Résumé" in Windows-1252
        assert_eq!(decode(&[0x52, 0xE9, 0x73, 0x75, 0x6D, 0xE9, 0x20, 0x64, 0x75, 0x20, 0x63, 0x6F, 0x6E, 0x74, 0x72, 0x61, 0x74]).unwrap(), "Résumé du contrat");
    }

    #[test]
    fn rejects_binary() {
        assert_eq!(decode(&[0x7F, 0x45, 0x4C, 0x46, 0x00, 0x01]), Err(SkipReason::Binary));
    }
}
