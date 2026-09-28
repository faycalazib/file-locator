//! E-books `.epub` (lot 5.3): the chapters' text in reading order.
//!
//! `META-INF/container.xml` points to the package (`.opf`), whose manifest
//! lists the files and whose spine gives the reading order. Each chapter is
//! XHTML: block elements become lines, `head`, `script` and `style` are
//! left out.

use std::collections::HashMap;
use std::io::{Cursor, Read};

use quick_xml::events::Event;
use quick_xml::Reader;

use super::odf::entity;
use super::SkipReason;

/// Total text read from one book (bomb guard on the chapters).
const MAX_BOOK_BYTES: u64 = 200 * 1024 * 1024;

pub fn extract(bytes: &[u8]) -> Result<String, SkipReason> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| SkipReason::Corrupt)?;
    if archive.by_name("META-INF/encryption.xml").is_ok() {
        return Err(SkipReason::Encrypted);
    }
    let mut read = |name: &str, budget: &mut u64| -> Option<String> {
        let entry = archive.by_name(name).ok()?;
        let mut text = String::new();
        entry.take(*budget).read_to_string(&mut text).ok()?;
        *budget = budget.saturating_sub(text.len() as u64);
        Some(text)
    };
    let mut budget = MAX_BOOK_BYTES;
    let container = read("META-INF/container.xml", &mut budget).ok_or(SkipReason::Corrupt)?;
    let opf_path = attribute_of(&container, "rootfile", "full-path").ok_or(SkipReason::Corrupt)?;
    let opf = read(&opf_path, &mut budget).ok_or(SkipReason::Corrupt)?;
    let base = opf_path.rsplit_once('/').map_or(String::new(), |(dir, _)| format!("{dir}/"));

    let mut out = String::new();
    for href in spine(&opf) {
        if budget == 0 {
            break;
        }
        let path = format!("{base}{}", percent_decode(href.split('#').next().unwrap_or(&href)));
        if let Some(xhtml) = read(&path, &mut budget) {
            out.push_str(&chapter_text(&xhtml));
            out.push_str("\n\n");
        }
    }
    Ok(out.lines().map(str::trim_end).collect::<Vec<_>>().join("\n").trim().to_owned())
}

/// First value of `attr` on an element named `tag` (any namespace prefix).
fn attribute_of(xml: &str, tag: &str, attr: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event().ok()? {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == tag => {
                return e
                    .attributes()
                    .flatten()
                    .find(|a| a.key.local_name().as_ref() == attr)
                    .map(|a| a.value.into_owned());
            }
            Event::Eof => return None,
            _ => {}
        }
    }
}

/// The chapters' `href`s, in the spine's order.
fn spine(opf: &str) -> Vec<String> {
    let mut reader = Reader::from_str(opf);
    let mut items: HashMap<String, String> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let attr = |name: &str| {
                    e.attributes().flatten().find(|a| a.key.local_name().as_ref() == name).map(|a| a.value.into_owned())
                };
                match e.local_name().as_ref() {
                    "item" => {
                        if let (Some(id), Some(href)) = (attr("id"), attr("href")) {
                            items.insert(id, href);
                        }
                    }
                    "itemref" => order.extend(attr("idref")),
                    _ => {}
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    order.into_iter().filter_map(|id| items.get(&id).cloned()).collect()
}

/// Visible text of one XHTML chapter.
fn chapter_text(xhtml: &str) -> String {
    const BLOCKS: &[&str] = &["p", "div", "h1", "h2", "h3", "h4", "h5", "h6", "li", "tr", "blockquote", "section"];
    let mut reader = Reader::from_str(xhtml);
    let mut out = String::new();
    let mut hidden = 0usize;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                "head" | "script" | "style" => hidden += 1,
                name if BLOCKS.contains(&name) => out.push('\n'),
                _ => {}
            },
            Ok(Event::Empty(e)) if matches!(e.local_name().as_ref(), "br" | "hr") => out.push('\n'),
            Ok(Event::End(e)) => match e.local_name().as_ref() {
                "head" | "script" | "style" => hidden = hidden.saturating_sub(1),
                "td" | "th" => out.push('\t'),
                _ => {}
            },
            Ok(Event::Text(t)) if hidden == 0 => out.push_str(&t.html_content()),
            Ok(Event::GeneralRef(r)) if hidden == 0 => out.push(entity(&r)),
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    out
}

/// `chapitre%201.xhtml` → `chapitre 1.xhtml` (hrefs are URLs).
fn percent_decode(href: &str) -> String {
    let bytes = href.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if let Some(b) = bytes.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spine_order_and_chapter_text() {
        let opf = r#"<package><manifest><item id="c2" href="ch2.xhtml"/><item id="c1" href="chapitre%201.xhtml"/></manifest>
<spine><itemref idref="c1"/><itemref idref="c2"/></spine></package>"#;
        assert_eq!(spine(opf), ["chapitre%201.xhtml", "ch2.xhtml"]);
        assert_eq!(percent_decode("chapitre%201.xhtml"), "chapitre 1.xhtml");
        let html = "<html><head><title>T</title><style>p{}</style></head><body><h1>Le phare</h1><p>Un gardien&nbsp;seul.</p></body></html>";
        assert_eq!(chapter_text(html).trim(), "Le phare\nUn gardien seul.");
    }
}
