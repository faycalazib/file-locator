//! PowerPoint 97-2003 `.ppt` (lot 5.3), from the [MS-PPT] specification.
//!
//! The `PowerPoint Document` stream is a tree of records (8-byte header:
//! version/instance, type, length; version 0xF = container). Text lives in
//! two places:
//! - the placeholders (title, body) in the `SlideListWithText` of the
//!   document, one `SlidePersistAtom` per slide, followed by its texts;
//! - free text boxes, inside each `Slide` container's drawing.
//!
//! Both are merged slide by slide, in the same `— N —` blocks as `.pptx`
//! (so the preview shows slide cards). Masters (layout templates full of
//! "Click to edit…" placeholders) are skipped.

use std::io::{Cursor, Read};

use super::SkipReason;

const SLIDE: u16 = 0x03EE;
const NOTES: u16 = 0x03F0;
const MAIN_MASTER: u16 = 0x03F8;
const SLIDE_PERSIST_ATOM: u16 = 0x03F3;
const SLIDE_LIST_WITH_TEXT: u16 = 0x0FF0;
const TEXT_CHARS_ATOM: u16 = 0x0FA0;
const TEXT_BYTES_ATOM: u16 = 0x0FA8;

pub fn extract(bytes: &[u8]) -> Result<String, SkipReason> {
    let mut cfb = cfb::CompoundFile::open(Cursor::new(bytes)).map_err(|_| SkipReason::Corrupt)?;
    if cfb.exists("EncryptedSummary") {
        return Err(SkipReason::Encrypted);
    }
    let mut stream = cfb.open_stream("PowerPoint Document").map_err(|_| SkipReason::Corrupt)?;
    let mut data = Vec::new();
    stream.read_to_end(&mut data).map_err(|_| SkipReason::Corrupt)?;

    let mut state = Walk::default();
    walk(&data, 0, data.len(), &mut state, 0);
    let slides = state.merge();
    if slides.is_empty() {
        return Ok(String::new());
    }
    let mut out = String::new();
    for (i, texts) in slides.iter().enumerate() {
        out.push_str(&format!("— {} —\n", i + 1));
        for text in texts {
            out.push_str(text);
            out.push('\n');
        }
        out.push('\n');
    }
    Ok(out.trim_end().to_owned())
}

#[derive(Default)]
struct Walk {
    /// Placeholder texts, one list per slide (from `SlideListWithText`).
    listed: Vec<Vec<String>>,
    /// Text-box texts, one list per `Slide` container, in file order.
    drawn: Vec<Vec<String>>,
}

impl Walk {
    fn merge(self) -> Vec<Vec<String>> {
        let count = self.listed.len().max(self.drawn.len());
        (0..count)
            .map(|i| {
                let mut texts = self.listed.get(i).cloned().unwrap_or_default();
                texts.extend(self.drawn.get(i).cloned().unwrap_or_default());
                texts
            })
            .filter(|texts: &Vec<String>| !texts.is_empty())
            .collect()
    }
}

/// Where a text atom sits.
#[derive(Clone, Copy, PartialEq)]
enum Place {
    Elsewhere,
    /// `SlideListWithText` of the slides (instance 0).
    SlideList,
    /// Inside a `Slide` container.
    Slide,
    /// Masters and notes: not slide content.
    Skipped,
}

fn walk(data: &[u8], start: usize, end: usize, state: &mut Walk, depth: usize) {
    walk_in(data, start, end, state, depth, Place::Elsewhere);
}

fn walk_in(data: &[u8], mut at: usize, end: usize, state: &mut Walk, depth: usize, place: Place) {
    // Records nest only a few levels; the limit guards against loops in damaged files.
    if depth > 32 {
        return;
    }
    while at + 8 <= end {
        let ver_inst = u16::from_le_bytes([data[at], data[at + 1]]);
        let kind = u16::from_le_bytes([data[at + 2], data[at + 3]]);
        let len = u32::from_le_bytes([data[at + 4], data[at + 5], data[at + 6], data[at + 7]]) as usize;
        let body = at + 8;
        let Some(body_end) = body.checked_add(len).filter(|&e| e <= end) else { return };
        let is_container = ver_inst & 0x000F == 0x000F;
        if is_container {
            let inner = match kind {
                MAIN_MASTER | NOTES => Place::Skipped,
                SLIDE if place != Place::Skipped => {
                    state.drawn.push(Vec::new());
                    Place::Slide
                }
                // Instance 0 = the slides' list (1 = masters, 2 = notes).
                SLIDE_LIST_WITH_TEXT if ver_inst >> 4 == 0 => Place::SlideList,
                SLIDE_LIST_WITH_TEXT => Place::Skipped,
                _ => place,
            };
            walk_in(data, body, body_end, state, depth + 1, inner);
        } else {
            match kind {
                SLIDE_PERSIST_ATOM if place == Place::SlideList => state.listed.push(Vec::new()),
                TEXT_CHARS_ATOM | TEXT_BYTES_ATOM => {
                    let raw = &data[body..body_end];
                    let text = if kind == TEXT_CHARS_ATOM {
                        let units: Vec<u16> = raw.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
                        String::from_utf16_lossy(&units)
                    } else {
                        // "Compressed" text: the low byte of each UTF-16 unit.
                        raw.iter().map(|&b| char::from(b)).collect()
                    };
                    let text = text.replace(['\r', '\u{0B}'], "\n").trim().to_owned();
                    if !text.is_empty() {
                        let target = match place {
                            Place::SlideList => state.listed.last_mut(),
                            Place::Slide => state.drawn.last_mut(),
                            _ => None,
                        };
                        if let Some(target) = target {
                            target.push(text);
                        }
                    }
                }
                _ => {}
            }
        }
        at = body_end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(ver_inst: u16, kind: u16, body: &[u8]) -> Vec<u8> {
        let mut out = ver_inst.to_le_bytes().to_vec();
        out.extend(kind.to_le_bytes());
        out.extend((body.len() as u32).to_le_bytes());
        out.extend(body);
        out
    }

    fn chars(text: &str) -> Vec<u8> {
        record(0, TEXT_CHARS_ATOM, &text.encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>())
    }

    #[test]
    fn placeholders_and_text_boxes_merge_per_slide_masters_skipped() {
        let list = [record(0, SLIDE_PERSIST_ATOM, &[0; 20]), chars("Planning"), record(0, SLIDE_PERSIST_ATOM, &[0; 20]), chars("Budget")].concat();
        let document = [record(0x000F, SLIDE_LIST_WITH_TEXT, &list)].concat();
        let master = record(0x000F, MAIN_MASTER, &chars("Click to edit Master title style"));
        let slide1 = record(0x000F, SLIDE, &[]);
        let slide2 = record(0x000F, SLIDE, &record(0x000F, 0xF00D, &chars("Note : échafaudage")));
        let stream = [record(0x000F, 0x03E8, &document), master, slide1, slide2].concat();
        let mut state = Walk::default();
        walk(&stream, 0, stream.len(), &mut state, 0);
        assert_eq!(state.merge(), vec![vec!["Planning".to_owned()], vec!["Budget".to_owned(), "Note : échafaudage".to_owned()]]);
    }
}
