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
//!
//! A "fast save" appends the changed records instead of rewriting the file:
//! old versions of the document and its slides stay in the stream (BUG-045).
//! The live version is found through the edit chain: `Current User` →
//! `UserEditAtom` → persist directories → the document and each slide. A
//! file without that chain (damaged) is read from start to end.

use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};

use super::SkipReason;

const SLIDE: u16 = 0x03EE;
const NOTES: u16 = 0x03F0;
const MAIN_MASTER: u16 = 0x03F8;
const SLIDE_PERSIST_ATOM: u16 = 0x03F3;
const SLIDE_LIST_WITH_TEXT: u16 = 0x0FF0;
const TEXT_CHARS_ATOM: u16 = 0x0FA0;
const TEXT_BYTES_ATOM: u16 = 0x0FA8;
const USER_EDIT_ATOM: u16 = 0x0FF5;
const PERSIST_DIRECTORY_ATOM: u16 = 0x1772;

pub fn extract(bytes: &[u8]) -> Result<String, SkipReason> {
    let mut cfb = cfb::CompoundFile::open(Cursor::new(bytes)).map_err(|_| SkipReason::Corrupt)?;
    if cfb.exists("EncryptedSummary") {
        return Err(SkipReason::Encrypted);
    }
    let mut stream = cfb.open_stream("PowerPoint Document").map_err(|_| SkipReason::Corrupt)?;
    let mut data = Vec::new();
    stream.read_to_end(&mut data).map_err(|_| SkipReason::Corrupt)?;

    let slides = match live(&mut cfb, &data) {
        Some(live) => live_slides(&data, &live),
        None => {
            let mut state = Walk::default();
            walk(&data, 0, data.len(), &mut state, 0);
            state.merge()
        }
    };
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

fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(data.get(at..at.checked_add(2)?)?.try_into().ok()?))
}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at.checked_add(4)?)?.try_into().ok()?))
}

/// End of the record that starts at `at`.
fn record_end(data: &[u8], at: usize) -> Option<usize> {
    let end = at.checked_add(8)?.checked_add(u32_at(data, at + 4)? as usize)?;
    (end <= data.len()).then_some(end)
}

/// The live version: where the document container is, and every persist
/// object (slides…) by its id. Newer edits override older ones.
struct Live {
    document: usize,
    persist: HashMap<u32, usize>,
}

fn live<F: Read + std::io::Seek>(cfb: &mut cfb::CompoundFile<F>, data: &[u8]) -> Option<Live> {
    let mut current = Vec::new();
    cfb.open_stream("Current User").ok()?.read_to_end(&mut current).ok()?;
    // CurrentUserAtom: record header, size, headerToken, offsetToCurrentEdit.
    let mut edit = u32_at(&current, 16)? as usize;
    let mut directories = Vec::new();
    let mut document_ref = None;
    let mut seen = HashSet::new();
    while seen.insert(edit) {
        if u16_at(data, edit + 2)? != USER_EDIT_ATOM {
            return None;
        }
        // UserEditAtom: lastSlideIdRef, version, minor, major, offsetLastEdit,
        // offsetPersistDirectory, docPersistIdRef.
        let body = edit + 8;
        directories.push(u32_at(data, body + 12)? as usize);
        document_ref.get_or_insert(u32_at(data, body + 16)?);
        match u32_at(data, body + 8)? {
            0 => break,
            previous => edit = previous as usize,
        }
    }
    let mut persist = HashMap::new();
    for &directory in directories.iter().rev() {
        if u16_at(data, directory + 2)? != PERSIST_DIRECTORY_ATOM {
            return None;
        }
        let end = record_end(data, directory)?;
        let mut at = directory + 8;
        // Entries: 20-bit first id + 12-bit count, then `count` offsets.
        while at + 4 <= end {
            let entry = u32_at(data, at)?;
            at += 4;
            for i in 0..entry >> 20 {
                persist.insert((entry & 0xF_FFFF) + i, u32_at(data, at)? as usize);
                at += 4;
            }
        }
    }
    Some(Live { document: *persist.get(&document_ref?)?, persist })
}

/// Slides of the live version, in the order of the document's slide list.
fn live_slides(data: &[u8], live: &Live) -> Vec<Vec<String>> {
    let mut state = Walk::default();
    if let Some(end) = record_end(data, live.document) {
        walk(data, live.document, end, &mut state, 0);
    }
    state
        .listed
        .into_iter()
        .map(|(persist_ref, mut texts)| {
            if let Some((&at, end)) = live.persist.get(&persist_ref).and_then(|at| Some((at, record_end(data, *at)?))) {
                let mut slide = Walk::default();
                walk(data, at, end, &mut slide, 0);
                texts.extend(slide.drawn.into_iter().flatten());
            }
            texts
        })
        .filter(|texts| !texts.is_empty())
        .collect()
}

#[derive(Default)]
struct Walk {
    /// Placeholder texts, one list per slide (from `SlideListWithText`), with
    /// the slide's persist id.
    listed: Vec<(u32, Vec<String>)>,
    /// Text-box texts, one list per `Slide` container, in file order.
    drawn: Vec<Vec<String>>,
}

impl Walk {
    fn merge(self) -> Vec<Vec<String>> {
        let count = self.listed.len().max(self.drawn.len());
        (0..count)
            .map(|i| {
                let mut texts = self.listed.get(i).map(|(_, t)| t.clone()).unwrap_or_default();
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
                SLIDE_PERSIST_ATOM if place == Place::SlideList => state.listed.push((u32_at(data, body).unwrap_or(0), Vec::new())),
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
                            Place::SlideList => state.listed.last_mut().map(|(_, t)| t),
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

    /// A fast save: the first version (one slide "Ancien titre") is followed by
    /// a second document and a new slide; only the live version is read.
    #[test]
    fn fast_saves_read_only_the_live_version() {
        let persist = |id: u32| record(0, SLIDE_PERSIST_ATOM, &[id.to_le_bytes().as_slice(), &[0; 16]].concat());
        let document = |slides: &[(u32, &str)]| {
            let list: Vec<u8> = slides.iter().flat_map(|(id, title)| [persist(*id), chars(title)].concat()).collect();
            record(0x000F, 0x03E8, &record(0x000F, SLIDE_LIST_WITH_TEXT, &list))
        };
        let slide = |text: &str| record(0x000F, SLIDE, &record(0x000F, 0xF00D, &chars(text)));
        let directory = |entries: &[(u32, usize)]| {
            let body: Vec<u8> = entries.iter().flat_map(|(id, at)| [(id | 1 << 20).to_le_bytes(), (*at as u32).to_le_bytes()].concat()).collect();
            record(0, PERSIST_DIRECTORY_ATOM, &body)
        };
        let edit = |last: usize, dir: usize, doc: u32| {
            let body = [0u32.to_le_bytes(), [0, 0, 0, 3], (last as u32).to_le_bytes(), (dir as u32).to_le_bytes(), doc.to_le_bytes(), 9u32.to_le_bytes(), [0; 4]].concat();
            record(0, USER_EDIT_ATOM, &body)
        };
        let mut stream = Vec::new();
        // Version 1: document 1, slide 2 ("Ancien titre" / "Vieille note").
        let doc1 = stream.len();
        stream.extend(document(&[(2, "Ancien titre")]));
        let slide1 = stream.len();
        stream.extend(slide("Vieille note"));
        let dir1 = stream.len();
        stream.extend(directory(&[(1, doc1), (2, slide1)]));
        let edit1 = stream.len();
        stream.extend(edit(0, dir1, 1));
        // Version 2 (fast save): new document and new slide 2, same ids.
        let doc2 = stream.len();
        stream.extend(document(&[(2, "Planning"), (3, "Budget")]));
        let slide2 = stream.len();
        stream.extend(slide("Note : échafaudage"));
        let dir2 = stream.len();
        stream.extend(directory(&[(1, doc2), (2, slide2)]));
        let edit2 = stream.len();
        stream.extend(edit(edit1, dir2, 1));

        let mut cfb = cfb::CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        std::io::Write::write_all(&mut cfb.create_stream("PowerPoint Document").unwrap(), &stream).unwrap();
        let current = [record(0, 0x0FF6, &[]).as_slice(), &20u32.to_le_bytes(), &0xE391_C05Fu32.to_le_bytes(), &(edit2 as u32).to_le_bytes()].concat();
        std::io::Write::write_all(&mut cfb.create_stream("Current User").unwrap(), &current).unwrap();
        let bytes = cfb.into_inner().into_inner();

        let text = extract(&bytes).unwrap();
        assert_eq!(text, "— 1 —\nPlanning\nNote : échafaudage\n\n— 2 —\nBudget");
    }
}
