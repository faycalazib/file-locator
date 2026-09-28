//! OpenDocument text and presentations (`.odt`, `.odp`, lot 5.3): the text
//! of `content.xml`. Spreadsheets (`.ods`) are read by calamine (office.rs).
//!
//! Paragraphs and headings become lines; `text:s` (spaces), `text:tab` and
//! `text:line-break` are kept. In a presentation each `draw:page` becomes a
//! `— N —` block, like `.pptx`, and speaker notes are left out.

use std::io::{Cursor, Read};

use quick_xml::events::Event;
use quick_xml::Reader;

use super::SkipReason;

const MAX_XML_BYTES: u64 = 200 * 1024 * 1024;

pub fn extract(bytes: &[u8]) -> Result<String, SkipReason> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| SkipReason::Corrupt)?;
    // Encrypted OpenDocument files declare it in their manifest.
    if let Ok(mut manifest) = archive.by_name("META-INF/manifest.xml") {
        let mut xml = String::new();
        if manifest.read_to_string(&mut xml).is_ok() && xml.contains("encryption-data") {
            return Err(SkipReason::Encrypted);
        }
    }
    let entry = archive.by_name("content.xml").map_err(|_| SkipReason::Corrupt)?;
    if entry.size() > MAX_XML_BYTES {
        return Err(SkipReason::TooLarge);
    }
    let mut xml = String::new();
    entry.take(MAX_XML_BYTES).read_to_string(&mut xml).map_err(|_| SkipReason::Corrupt)?;
    parse_content(&xml)
}

fn parse_content(xml: &str) -> Result<String, SkipReason> {
    let mut reader = Reader::from_str(xml);
    let mut out = String::new();
    let mut pages = 0usize;
    // Inside `presentation:notes`, `office:annotation`… (not shown text).
    let mut hidden = 0usize;
    // Inside a paragraph or heading: text events belong to the document.
    let mut in_paragraph = 0usize;

    loop {
        match reader.read_event().map_err(|_| SkipReason::Corrupt)? {
            Event::Start(e) => match e.name().as_ref() {
                "draw:page" => {
                    pages += 1;
                    out.push_str(&format!("\n\n— {pages} —\n"));
                }
                "presentation:notes" | "office:annotation" | "text:tracked-changes" => hidden += 1,
                "text:p" | "text:h" => {
                    in_paragraph += 1;
                    if hidden == 0 && !out.ends_with('\n') {
                        out.push('\n');
                    }
                }
                _ => {}
            },
            Event::Empty(e) if hidden == 0 => match e.name().as_ref() {
                "text:s" => {
                    let count = e
                        .attributes()
                        .flatten()
                        .find(|a| a.key.as_ref() == "text:c")
                        .and_then(|a| a.value.parse::<usize>().ok())
                        .unwrap_or(1);
                    out.push_str(&" ".repeat(count.min(64)));
                }
                "text:tab" => out.push('\t'),
                "text:line-break" | "text:p" | "text:h" => out.push('\n'),
                _ => {}
            },
            Event::End(e) => match e.name().as_ref() {
                "presentation:notes" | "office:annotation" | "text:tracked-changes" => hidden = hidden.saturating_sub(1),
                "text:p" | "text:h" => in_paragraph = in_paragraph.saturating_sub(1),
                _ => {}
            },
            Event::Text(t) if hidden == 0 && in_paragraph > 0 => out.push_str(&t.html_content()),
            Event::GeneralRef(r) if hidden == 0 && in_paragraph > 0 => out.push(entity(&r)),
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(out.lines().map(str::trim_end).collect::<Vec<_>>().join("\n").trim().to_owned())
}

/// `&amp;`, `&#233;`… (XML defines five named entities).
pub fn entity(r: &quick_xml::events::BytesRef) -> char {
    if let Ok(Some(c)) = r.resolve_char_ref() {
        return c;
    }
    match &**r {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        _ => ' ',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs_spaces_and_slides() {
        let xml = r#"<office:document-content><office:body><office:presentation>
<draw:page><draw:frame><draw:text-box><text:p>Planning <text:span>du</text:span><text:s text:c="2"/>chantier</text:p></draw:text-box></draw:frame>
<presentation:notes><text:p>notes cachées</text:p></presentation:notes></draw:page>
<draw:page><draw:frame><draw:text-box><text:p>Budget<text:tab/>2024 &amp; plus</text:p></draw:text-box></draw:frame></draw:page>
</office:presentation></office:body></office:document-content>"#;
        assert_eq!(parse_content(xml).unwrap(), "— 1 —\nPlanning du  chantier\n\n— 2 —\nBudget\t2024 & plus");
    }
}
