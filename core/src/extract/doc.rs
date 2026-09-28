//! Word 97-2003 `.doc` (lot 5.3), from the [MS-DOC] specification.
//!
//! A `.doc` is a compound file ("CFB"). Its `WordDocument` stream starts with
//! the FIB, which gives the position of the piece table (CLX) in the table
//! stream (`0Table` or `1Table`). Each piece is a run of text stored either
//! as 8-bit Windows-1252 ("compressed") or as UTF-16.

use std::io::{Cursor, Read};

use super::SkipReason;

/// FIB flags (`FibBase`, offset 10).
const F_ENCRYPTED: u16 = 0x0100;
const F_WHICH_TABLE: u16 = 0x0200;
/// `fcClx` is the 34th pair of `FibRgFcLcb97`.
const CLX_PAIR: usize = 33;

pub fn extract(bytes: &[u8]) -> Result<String, SkipReason> {
    let mut cfb = cfb::CompoundFile::open(Cursor::new(bytes)).map_err(|_| SkipReason::Corrupt)?;
    let word = read_stream(&mut cfb, "WordDocument")?;
    let u16_at = |data: &[u8], at: usize| data.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |data: &[u8], at: usize| data.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));

    if u16_at(&word, 0) != Some(0xA5EC) {
        return Err(SkipReason::Corrupt);
    }
    // Word 6/95 files (nFib < 101) use another layout.
    if u16_at(&word, 2).is_none_or(|n| n < 101) {
        return Err(SkipReason::Unsupported);
    }
    let flags = u16_at(&word, 10).ok_or(SkipReason::Corrupt)?;
    if flags & F_ENCRYPTED != 0 {
        return Err(SkipReason::Encrypted);
    }
    // FibBase (32) + csw + fibRgW + cslw + fibRgLw + cbRgFcLcb, then the pairs.
    let csw = usize::from(u16_at(&word, 32).ok_or(SkipReason::Corrupt)?);
    let lw_at = 34 + csw * 2;
    let cslw = usize::from(u16_at(&word, lw_at).ok_or(SkipReason::Corrupt)?);
    let fc_lcb_at = lw_at + 2 + cslw * 4 + 2;
    let fc_clx = u32_at(&word, fc_lcb_at + CLX_PAIR * 8).ok_or(SkipReason::Corrupt)? as usize;
    let lcb_clx = u32_at(&word, fc_lcb_at + CLX_PAIR * 8 + 4).ok_or(SkipReason::Corrupt)? as usize;

    let table = read_stream(&mut cfb, if flags & F_WHICH_TABLE != 0 { "1Table" } else { "0Table" })?;
    let clx = table.get(fc_clx..fc_clx + lcb_clx).ok_or(SkipReason::Corrupt)?;
    let pieces = piece_table(clx).ok_or(SkipReason::Corrupt)?;

    let mut text = String::new();
    for (chars, fc) in pieces {
        let compressed = fc & 0x4000_0000 != 0;
        let fc = (fc & 0x3FFF_FFFF) as usize;
        if compressed {
            let start = fc / 2;
            let Some(run) = word.get(start..start + chars) else { continue };
            text.push_str(&encoding_rs::WINDOWS_1252.decode_without_bom_handling(run).0);
        } else {
            let Some(run) = word.get(fc..fc + chars * 2) else { continue };
            let units: Vec<u16> = run.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
            text.push_str(&String::from_utf16_lossy(&units));
        }
    }
    Ok(clean(&text))
}

fn read_stream<F: std::io::Read + std::io::Seek>(cfb: &mut cfb::CompoundFile<F>, name: &str) -> Result<Vec<u8>, SkipReason> {
    let mut stream = cfb.open_stream(name).map_err(|_| SkipReason::Corrupt)?;
    let mut data = Vec::new();
    stream.read_to_end(&mut data).map_err(|_| SkipReason::Corrupt)?;
    Ok(data)
}

/// The CLX: optional `Prc` blocks (type 1), then the `Pcdt` (type 2) holding
/// the piece table: n+1 character positions, then n 8-byte descriptors.
/// Returns (character count, `fc`) per piece.
fn piece_table(clx: &[u8]) -> Option<Vec<(usize, u32)>> {
    let mut at = 0;
    while clx.get(at) == Some(&1) {
        let size = u16::from_le_bytes([*clx.get(at + 1)?, *clx.get(at + 2)?]) as usize;
        at += 3 + size;
    }
    if clx.get(at) != Some(&2) {
        return None;
    }
    let size = u32::from_le_bytes(clx.get(at + 1..at + 5)?.try_into().ok()?) as usize;
    let plc = clx.get(at + 5..at + 5 + size)?;
    // (n + 1) * 4 + n * 8 = size
    let n = size.checked_sub(4)? / 12;
    let cp = |i: usize| -> Option<u32> { Some(u32::from_le_bytes(plc.get(i * 4..i * 4 + 4)?.try_into().ok()?)) };
    let mut pieces = Vec::with_capacity(n);
    for i in 0..n {
        let chars = cp(i + 1)?.checked_sub(cp(i)?)? as usize;
        let pcd = (n + 1) * 4 + i * 8;
        let fc = u32::from_le_bytes(plc.get(pcd + 2..pcd + 6)?.try_into().ok()?);
        pieces.push((chars, fc));
    }
    Some(pieces)
}

/// Word's special characters: paragraph and cell marks, and field codes
/// (`{ HYPERLINK "…" }` instructions are dropped, their results kept).
fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    // Depth of fields whose instruction part is being skipped.
    let mut hidden = 0usize;
    let mut fields: Vec<bool> = Vec::new();
    for c in text.chars() {
        match c {
            '\u{13}' => {
                fields.push(true);
                hidden += 1;
            }
            '\u{14}' => {
                if fields.last() == Some(&true) {
                    *fields.last_mut().unwrap() = false;
                    hidden -= 1;
                }
            }
            '\u{15}' => {
                if fields.pop() == Some(true) {
                    hidden -= 1;
                }
            }
            _ if hidden > 0 => {}
            '\r' | '\u{0B}' | '\u{0C}' => out.push('\n'),
            // Cell mark: end of a table cell.
            '\u{07}' => out.push('\t'),
            // Pictures, footnote marks and other anchors.
            '\u{01}' | '\u{02}' | '\u{05}' | '\u{08}' => {}
            c => out.push(c),
        }
    }
    out.lines().map(str::trim_end).collect::<Vec<_>>().join("\n").trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_instructions_are_dropped_results_kept() {
        let raw = "Voir \u{13} HYPERLINK \"http://x\" \u{14}le site\u{15} ici.\rFin\u{07}cellule";
        assert_eq!(clean(raw), "Voir le site ici.\nFin\tcellule");
    }

    #[test]
    fn piece_table_with_a_formatting_block_first() {
        // Prc (type 1, 2 bytes of data), then Pcdt with one piece of 5 chars.
        let mut clx = vec![1, 2, 0, 0xAA, 0xBB, 2];
        let plc: Vec<u8> = [0u32.to_le_bytes(), 5u32.to_le_bytes()]
            .concat()
            .into_iter()
            .chain([0, 0])
            .chain(0x4000_0400u32.to_le_bytes())
            .chain([0, 0])
            .collect();
        clx.extend((plc.len() as u32).to_le_bytes());
        clx.extend(plc);
        assert_eq!(piece_table(&clx), Some(vec![(5, 0x4000_0400)]));
    }
}
