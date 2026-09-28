//! Rich Text Format `.rtf` (lot 5.3): the visible text of the document.
//!
//! Groups `{…}` nest the formatting state; `\par`, `\line`, `\tab` and `\cell`
//! become line breaks or tabs; `\uN` is a Unicode character followed by `\ucN`
//! fallback characters to skip; `\'hh` is a byte in the document's code page
//! (`\ansicpg`). Groups that are not text (fonts, colors, styles, pictures,
//! embedded objects, `{\*\…}` destinations) are skipped whole.

use super::SkipReason;

/// Destinations whose content is never document text.
const SKIPPED: &[&str] = &[
    "fonttbl", "colortbl", "stylesheet", "info", "pict", "object", "datastore", "themedata", "listtable",
    "listoverridetable", "rsidtbl", "generator", "latentstyles", "xmlnstbl", "mmathPr", "filetbl", "revtbl",
    "colorschememapping", "fldinst", "header", "footer", "headerl", "headerr", "footerl", "footerr", "wgrffmtfilter",
];

#[derive(Clone)]
struct Group {
    skip: bool,
    /// Fallback characters after `\uN` (`\ucN`, 1 by default).
    uc: usize,
}

pub fn extract(bytes: &[u8]) -> Result<String, SkipReason> {
    if !bytes.starts_with(b"{\\rtf") {
        return Err(SkipReason::Corrupt);
    }
    let mut encoding = encoding_rs::WINDOWS_1252;
    let mut out = String::new();
    let mut pending: Vec<u8> = Vec::new(); // `\'hh` bytes, decoded together (multi-byte code pages)
    let mut stack: Vec<Group> = vec![Group { skip: false, uc: 1 }];
    let mut skip_chars = 0usize; // fallback characters still to skip after `\uN`
    let mut i = 0;

    let flush = |pending: &mut Vec<u8>, out: &mut String, encoding: &'static encoding_rs::Encoding| {
        if !pending.is_empty() {
            out.push_str(&encoding.decode_without_bom_handling(pending).0);
            pending.clear();
        }
    };

    while i < bytes.len() {
        let c = bytes[i];
        let skip = stack.last().is_some_and(|g| g.skip);
        match c {
            b'{' => {
                flush(&mut pending, &mut out, encoding);
                let top = stack.last().cloned().unwrap_or(Group { skip: false, uc: 1 });
                // `{\*\dest …}`: an optional destination, never text.
                let starred = bytes.get(i + 1..i + 3) == Some(b"\\*");
                stack.push(Group { skip: top.skip || starred, uc: top.uc });
                i += 1;
            }
            b'}' => {
                flush(&mut pending, &mut out, encoding);
                if stack.len() > 1 {
                    stack.pop();
                }
                i += 1;
            }
            b'\\' => {
                let (word, param, next) = control(bytes, i + 1);
                i = next;
                if word != "'" {
                    flush(&mut pending, &mut out, encoding);
                }
                match word.as_str() {
                    "'" => {
                        if let Some(byte) = bytes.get(i..i + 2).and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                            i += 2;
                            if skip_chars > 0 {
                                skip_chars -= 1;
                            } else if !skip {
                                pending.push(byte);
                            }
                        }
                    }
                    "u" => {
                        if let Some(n) = param {
                            if !skip {
                                let code = if n < 0 { n + 65536 } else { n } as u32;
                                out.extend(char::from_u32(code));
                            }
                            skip_chars = stack.last().map_or(1, |g| g.uc);
                        }
                    }
                    "uc" => {
                        if let (Some(g), Some(n)) = (stack.last_mut(), param) {
                            g.uc = n.max(0) as usize;
                        }
                    }
                    "ansicpg" => {
                        if let Some(enc) = param.and_then(code_page) {
                            encoding = enc;
                        }
                    }
                    "par" | "line" | "sect" | "page" | "row" if !skip => out.push('\n'),
                    "tab" | "cell" if !skip => out.push('\t'),
                    "emdash" if !skip => out.push('—'),
                    "endash" if !skip => out.push('–'),
                    "lquote" if !skip => out.push('‘'),
                    "rquote" if !skip => out.push('’'),
                    "ldblquote" if !skip => out.push('“'),
                    "rdblquote" if !skip => out.push('”'),
                    "bullet" if !skip => out.push('•'),
                    "~" if !skip => out.push('\u{A0}'),
                    "\\" | "{" | "}" if !skip => out.push_str(&word),
                    w if SKIPPED.contains(&w) => {
                        if let Some(g) = stack.last_mut() {
                            g.skip = true;
                        }
                    }
                    _ => {}
                }
            }
            b'\r' | b'\n' => i += 1,
            _ => {
                if skip_chars > 0 {
                    skip_chars -= 1;
                } else if !skip {
                    flush(&mut pending, &mut out, encoding);
                    out.push(char::from(c));
                }
                i += 1;
            }
        }
    }
    flush(&mut pending, &mut out, encoding);
    Ok(out.lines().map(str::trim_end).collect::<Vec<_>>().join("\n").trim().to_owned())
}

/// A control word after `\`: (word, numeric parameter, next index). A
/// control symbol (`\'`, `\\`, `\~`…) is returned as its single character.
fn control(bytes: &[u8], mut i: usize) -> (String, Option<i32>, usize) {
    let Some(&first) = bytes.get(i) else {
        return (String::new(), None, i);
    };
    if !first.is_ascii_alphabetic() {
        return ((first as char).to_string(), None, i + 1);
    }
    let start = i;
    while bytes.get(i).is_some_and(u8::is_ascii_alphabetic) {
        i += 1;
    }
    let word = String::from_utf8_lossy(&bytes[start..i]).into_owned();
    let num_start = i;
    if bytes.get(i) == Some(&b'-') {
        i += 1;
    }
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let param = std::str::from_utf8(&bytes[num_start..i]).ok().and_then(|s| s.parse().ok());
    // One space after a control word belongs to it.
    if bytes.get(i) == Some(&b' ') {
        i += 1;
    }
    (word, param, i)
}

fn code_page(cp: i32) -> Option<&'static encoding_rs::Encoding> {
    encoding_rs::Encoding::for_label(match cp {
        1250 => b"windows-1250",
        1251 => b"windows-1251",
        1252 => b"windows-1252",
        1253 => b"windows-1253",
        1254 => b"windows-1254",
        1255 => b"windows-1255",
        1256 => b"windows-1256",
        1257 => b"windows-1257",
        932 => b"shift_jis",
        936 => b"gbk",
        949 => b"euc-kr",
        950 => b"big5",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_accents_unicode_and_skipped_groups() {
        // `\uN?` built at run time: 1593, 1602, 1583 = ع ق د.
        let arabic: String = ["1593", "1602", "1583"].iter().map(|n| format!("{}u{n}?", '\\')).collect();
        let rtf = format!(
            "{}\n{}\n{}{} fin{}tab tableau {{{}b gras}}{}par}}",
            r"{\rtf1\ansi\ansicpg1252{\fonttbl{\f0 Arial;}}{\colortbl;\red0;}{\*\generator Word;}",
            r"\pard R\'e9sum\'e9 du g\'e9om\'e8tre\par",
            r"\uc1",
            arabic,
            '\\',
            '\\',
            '\\'
        );
        assert_eq!(extract(rtf.as_bytes()).unwrap(), "Résumé du géomètre\nعقد fin\ttableau gras");
    }

    #[test]
    fn not_an_rtf_file() {
        assert!(extract(b"hello").is_err());
    }
}
