//! Word (.docx): the text of `word/document.xml`, one line per paragraph.

use std::io::{Cursor, Read};

use quick_xml::events::Event;
use quick_xml::Reader;

use super::SkipReason;

/// `document.xml` above this size is not a normal document (zip bomb guard).
const MAX_XML_BYTES: u64 = 200 * 1024 * 1024;

pub fn extract(bytes: &[u8]) -> Result<String, SkipReason> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| SkipReason::Corrupt)?;
    // An encrypted .docx is an OLE container, not a zip: it fails above. A zip
    // without word/document.xml is not a Word document.
    let entry = archive.by_name("word/document.xml").map_err(|_| SkipReason::Corrupt)?;
    if entry.size() > MAX_XML_BYTES {
        return Err(SkipReason::TooLarge);
    }
    let mut xml = String::new();
    entry.take(MAX_XML_BYTES).read_to_string(&mut xml).map_err(|_| SkipReason::Corrupt)?;
    parse_document_xml(&xml)
}

fn parse_document_xml(xml: &str) -> Result<String, SkipReason> {
    let mut reader = Reader::from_str(xml);
    let mut out = String::new();
    let mut in_text = false;
    let mut paragraphs = 0usize;

    loop {
        match reader.read_event().map_err(|_| SkipReason::Corrupt)? {
            Event::Start(e) => match e.local_name().as_ref() {
                "t" => in_text = true,
                "p" => {
                    if paragraphs > 0 {
                        out.push('\n');
                    }
                    paragraphs += 1;
                }
                _ => {}
            },
            Event::Empty(e) => match e.local_name().as_ref() {
                "tab" => out.push('\t'),
                "br" | "cr" => out.push('\n'),
                "p" => {
                    if paragraphs > 0 {
                        out.push('\n');
                    }
                    paragraphs += 1;
                }
                _ => {}
            },
            Event::End(e) if e.local_name().as_ref() == "t" => in_text = false,
            Event::Text(t) if in_text => out.push_str(&t.html_content()),
            Event::GeneralRef(r) if in_text => {
                if let Ok(Some(c)) = r.resolve_char_ref() {
                    out.push(c);
                } else {
                    out.push(match &*r {
                        "amp" => '&',
                        "lt" => '<',
                        "gt" => '>',
                        "quot" => '"',
                        "apos" => '\'',
                        _ => ' ',
                    });
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs_tabs_and_entities() {
        let xml = r#"<w:document xmlns:w="x"><w:body>
            <w:p><w:r><w:t>Le présent </w:t></w:r><w:r><w:t>contrat</w:t></w:r></w:p>
            <w:p><w:r><w:t>Prix&#160;:</w:t><w:tab/><w:t>4 800 &amp; plus</w:t></w:r></w:p>
        </w:body></w:document>"#;
        assert_eq!(parse_document_xml(xml).unwrap(), "Le présent contrat\nPrix\u{a0}:\t4 800 & plus");
    }
}
