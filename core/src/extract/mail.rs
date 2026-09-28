//! E-mails (lots 2 and 5.4):
//!
//! - `.msg` (Outlook, msg_parser) and `.eml` (MIME, mail-parser): the
//!   message, then **each attachment as its own document**
//!   (`mail.msg › contrat.pdf`), read like an archive entry (an attached ZIP
//!   is opened too);
//! - `.mbox` (Thunderbird, Apple Mail…): every message, streamed;
//! - `.pst` / `.ost` (Microsoft's pure-Rust `outlook-pst`, no external tool):
//!   every message, `Inbox › Invoices › Re: contrat`, with its attachments.

use std::collections::HashMap;
use std::io::BufReader;
use std::path::Path;
use std::rc::Rc;

use mail_parser::{MessageParser, MimeHeaders};
use outlook_pst::ltp::prop_context::PropertyValue;
use outlook_pst::messaging::attachment::{AnsiAttachment, Attachment, AttachmentData, UnicodeAttachment};
use outlook_pst::messaging::folder::Folder;
use outlook_pst::messaging::message::{AnsiMessage, Message, UnicodeMessage};
use outlook_pst::messaging::store::{AnsiStore, EntryId, Store, UnicodeStore};
use outlook_pst::ndb::node_id::NodeId;
use outlook_pst::{AnsiPstFile, UnicodePstFile};

use super::archive::{attachment_docs, capture_done, capture_skips};
use super::{ExtractedDoc, SkipReason, INNER_SEP};
use crate::kind::FileKind;

/// Mailboxes deeper than this are cut (pathological folder trees).
const MAX_FOLDER_DEPTH: usize = 32;
/// Messages of an mbox larger than this are skipped (a message is read whole).
const MAX_MESSAGE_BYTES: usize = 64 * 1024 * 1024;

/// The text indexed for a message: subject, people, then the body; the
/// attachment names at the end (they are also documents of their own).
fn message_text(subject: &str, from: &str, to: &str, cc: &str, body: &str, attachments: &[String]) -> String {
    let mut out = format!("{subject}\n{from}\n");
    for people in [to, cc] {
        if !people.is_empty() {
            out.push_str(people);
            out.push('\n');
        }
    }
    out.push('\n');
    out.push_str(body.trim());
    for name in attachments.iter().filter(|n| !n.is_empty()) {
        out.push_str(&format!("\n📎 {name}"));
    }
    out
}

// ── .msg ──────────────────────────────────────────────────────────────

/// Text of a `.msg` (used when the message itself is inside an archive).
pub fn msg(bytes: &[u8]) -> Result<String, SkipReason> {
    msg_docs(bytes).map(|mut docs| docs.swap_remove(0).text)
}

/// A `.msg`: the message (first document, no inner path), then its attachments.
pub fn msg_docs(bytes: &[u8]) -> Result<Vec<ExtractedDoc>, SkipReason> {
    let mail = msg_parser::Outlook::from_slice(bytes).map_err(|_| SkipReason::Corrupt)?;
    let people = |list: &[msg_parser::Person]| {
        list.iter()
            .map(|p| if p.email.is_empty() { p.name.clone() } else { format!("{} <{}>", p.name, p.email) })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let names: Vec<String> = mail.attachments.iter().map(attachment_name_msg).collect();
    let text = message_text(
        &mail.subject,
        &format!("{} <{}>", mail.sender.name, mail.sender.email),
        &people(&mail.to),
        &people(&mail.cc),
        &mail.body,
        &names,
    );
    let mut out = vec![ExtractedDoc::whole(FileKind::Email, text)];
    for (attachment, name) in mail.attachments.into_iter().zip(names) {
        if !name.is_empty() && !attachment.payload_bytes.is_empty() {
            attachment_docs(&name, attachment.payload_bytes, None, "", &mut out);
        }
    }
    Ok(out)
}

fn attachment_name_msg(a: &msg_parser::Attachment) -> String {
    [&a.long_file_name, &a.file_name, &a.display_name]
        .into_iter()
        .find(|n| !n.trim().is_empty())
        .map(|n| clean_name(n))
        .unwrap_or_default()
}

/// An attachment name is one step of an inner path: no separators in it.
fn clean_name(name: &str) -> String {
    name.trim().trim_end_matches('\0').replace(['/', '\\'], "_").replace(INNER_SEP, " - ")
}

// ── .eml / .mbox ──────────────────────────────────────────────────────

/// Text of an `.eml` (inside an archive).
pub fn eml(bytes: &[u8]) -> Result<String, SkipReason> {
    eml_docs(bytes).map(|mut docs| docs.swap_remove(0).text)
}

/// An `.eml`: the message, then its attachments.
pub fn eml_docs(bytes: &[u8]) -> Result<Vec<ExtractedDoc>, SkipReason> {
    let (doc, attachments) = mime_message(bytes, "").ok_or(SkipReason::Corrupt)?;
    let mut out = vec![doc];
    out.extend(attachments);
    Ok(out)
}

/// One MIME message: its document (inner path `prefix`, or none) and the
/// documents of its attachments (under `prefix`).
fn mime_message(bytes: &[u8], prefix: &str) -> Option<(ExtractedDoc, Vec<ExtractedDoc>)> {
    let message = MessageParser::default().parse(bytes)?;
    let people = |address: Option<&mail_parser::Address>| {
        address
            .map(|a| {
                a.iter()
                    .map(|p| match (p.name(), p.address()) {
                        (Some(n), Some(e)) => format!("{n} <{e}>"),
                        (Some(n), None) => n.to_owned(),
                        (None, Some(e)) => e.to_owned(),
                        (None, None) => String::new(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default()
    };
    let subject = message.subject().unwrap_or_default().to_owned();
    // Plain text if the message has it, otherwise its HTML turned into text.
    let body = message.body_text(0).map(|b| b.into_owned()).unwrap_or_default();
    let parts: Vec<(String, Vec<u8>)> = message
        .attachments()
        .filter_map(|part| {
            let name = clean_name(part.attachment_name().unwrap_or_default());
            // An attached message (message/rfc822) without a name: its subject.
            let name = if name.is_empty() {
                part.message().and_then(|m| m.subject()).map(|s| format!("{}.eml", clean_name(s)))?
            } else {
                name
            };
            Some((name, part.contents().to_vec()))
        })
        .collect();
    let names: Vec<String> = parts.iter().map(|(n, _)| n.clone()).collect();
    let text = message_text(&subject, &people(message.from()), &people(message.to()), &people(message.cc()), &body, &names);
    let modified = message.date().and_then(|d| u64::try_from(d.to_timestamp()).ok());
    let doc = ExtractedDoc {
        inner: (!prefix.is_empty()).then(|| prefix.to_owned()),
        kind: FileKind::Email,
        size: Some(bytes.len() as u64),
        modified,
        text,
    };
    let mut attachments = Vec::new();
    for (name, data) in parts {
        attachment_docs(&name, data, modified, prefix, &mut attachments);
    }
    Some((doc, attachments))
}

/// Every message of an mbox file (`Subject`, `Subject (2)`…), streamed from disk.
pub fn mbox(path: &Path) -> Result<Vec<ExtractedDoc>, SkipReason> {
    let file = std::fs::File::open(path).map_err(|_| SkipReason::Unreadable)?;
    let mut out = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for message in mail_parser::mailbox::mbox::MessageIterator::new(BufReader::new(file)) {
        let Ok(message) = message else { break };
        let bytes = message.contents();
        if bytes.len() > MAX_MESSAGE_BYTES {
            continue;
        }
        let subject = MessageParser::default().parse(bytes).and_then(|m| m.subject().map(str::to_owned)).unwrap_or_default();
        let inner = unique(&mut seen, title(&subject));
        // Extraction (lot 6.7): only the message holding the wanted attachment.
        if capture_done() {
            break;
        }
        if capture_skips("", &inner) {
            continue;
        }
        if let Some((doc, attachments)) = mime_message(bytes, &inner) {
            out.push(doc);
            out.extend(attachments);
        }
    }
    if out.is_empty() {
        return Err(SkipReason::Corrupt);
    }
    Ok(out)
}

fn title(subject: &str) -> String {
    let t = clean_name(subject);
    if t.is_empty() { "—".to_owned() } else { t }
}

/// Two messages with the same subject in one folder keep distinct paths.
fn unique(seen: &mut HashMap<String, usize>, inner: String) -> String {
    let n = seen.entry(inner.clone()).or_insert(0);
    *n += 1;
    if *n > 1 { format!("{inner} ({n})") } else { inner }
}

// ── .pst / .ost ───────────────────────────────────────────────────────

/// Text of a PST string property. Subjects may start with a 2-char prefix
/// marker (0x01 + length): it is not part of the subject.
fn text(value: &PropertyValue, subject: bool) -> Option<String> {
    let raw = match value {
        PropertyValue::String8(v) => v.buffer().iter().map(|&b| char::from(b)).collect::<String>(),
        PropertyValue::Unicode(v) => String::from_utf16_lossy(v.buffer()),
        _ => return None,
    };
    let raw = raw.trim_end_matches('\0');
    let raw = if subject && raw.starts_with('\u{1}') { raw.chars().skip(2).collect() } else { raw.to_owned() };
    Some(raw)
}

/// FILETIME (100 ns since 1601) → Unix seconds.
fn filetime_to_unix(t: i64) -> Option<u64> {
    u64::try_from(t / 10_000_000 - 11_644_473_600).ok()
}

/// Attachments of a message: (file name, content).
type Files = Vec<(String, Vec<u8>)>;

/// The two on-disk flavours of PST/OST files. Attachments can only be read
/// through the typed messages.
enum Mailbox {
    Unicode(Rc<UnicodeStore>),
    Ansi(Rc<AnsiStore>),
}

impl Mailbox {
    fn open(path: &Path) -> Option<Self> {
        if let Ok(file) = UnicodePstFile::open(path) {
            return UnicodeStore::read(Rc::new(file)).ok().map(Mailbox::Unicode);
        }
        AnsiPstFile::open(path).ok().and_then(|f| AnsiStore::read(Rc::new(f)).ok()).map(Mailbox::Ansi)
    }

    fn store(&self) -> Rc<dyn Store> {
        match self {
            Mailbox::Unicode(s) => s.clone(),
            Mailbox::Ansi(s) => s.clone(),
        }
    }

    /// A message, and its attachments as (name, bytes).
    fn message(&self, entry_id: &EntryId) -> Option<(Rc<dyn Message>, Files)> {
        fn files<M: Message + ?Sized>(message: &M, read: impl Fn(NodeId) -> Option<Rc<dyn Attachment>>) -> Files {
            let Some(table) = message.attachment_table() else { return Vec::new() };
            table
                .rows_matrix()
                .filter_map(|row| {
                    let attachment = read(NodeId::from(u32::from(row.id())))?;
                    let props = attachment.properties();
                    // Long file name, short one, then the display name.
                    let name = [0x3707u16, 0x3704, 0x3001]
                        .iter()
                        .find_map(|id| props.get(*id).and_then(|v| text(v, false)).filter(|n| !n.trim().is_empty()))
                        .map(|n| clean_name(&n))?;
                    match attachment.data()? {
                        AttachmentData::Binary(data) => Some((name, data.buffer().to_vec())),
                        _ => None,
                    }
                })
                .collect()
        }
        match self {
            Mailbox::Unicode(store) => {
                let message = UnicodeMessage::read(store.clone(), entry_id, None).ok()?;
                let attached = files(message.as_ref(), |node| {
                    UnicodeAttachment::read(message.clone(), node, None).ok().map(|a| a as Rc<dyn Attachment>)
                });
                Some((message as Rc<dyn Message>, attached))
            }
            Mailbox::Ansi(store) => {
                let message = AnsiMessage::read(store.clone(), entry_id, None).ok()?;
                let attached = files(message.as_ref(), |node| {
                    AnsiAttachment::read(message.clone(), node, None).ok().map(|a| a as Rc<dyn Attachment>)
                });
                Some((message as Rc<dyn Message>, attached))
            }
        }
    }
}

/// Every message of a mailbox (and its attachments), one document each.
pub fn pst(path: &Path) -> Result<Vec<ExtractedDoc>, SkipReason> {
    let mailbox = Mailbox::open(path).ok_or(SkipReason::Corrupt)?;
    let store = mailbox.store();
    let root_id = store.properties().ipm_sub_tree_entry_id().map_err(|_| SkipReason::Corrupt)?;
    let root = store.open_folder(&root_id).map_err(|_| SkipReason::Corrupt)?;
    let mut out = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    walk(&mailbox, &root, None, 0, &mut out, &mut seen);
    Ok(out)
}

fn walk(
    mailbox: &Mailbox,
    folder: &Rc<dyn Folder>,
    prefix: Option<&str>,
    depth: usize,
    out: &mut Vec<ExtractedDoc>,
    seen: &mut HashMap<String, usize>,
) {
    if depth > MAX_FOLDER_DEPTH {
        return;
    }
    let store = mailbox.store();
    // The IPM subtree root ("Top of Personal Folders") is not shown.
    let here = match prefix {
        None => String::new(),
        Some(p) => {
            let name = folder.properties().display_name().unwrap_or_default();
            if p.is_empty() { name } else { format!("{p}{INNER_SEP}{name}") }
        }
    };

    if let Some(contents) = folder.contents_table() {
        for row in contents.rows_matrix() {
            let Ok(entry_id) = store.properties().make_entry_id(NodeId::from(u32::from(row.id()))) else { continue };
            let Some((message, attachments)) = mailbox.message(&entry_id) else { continue };
            let props = message.properties();
            let subject = props.get(0x0037).and_then(|v| text(v, true)).unwrap_or_default();
            let sender = props.get(0x0C1A).and_then(|v| text(v, false)).unwrap_or_default();
            let body = props.get(0x1000).and_then(|v| text(v, false)).unwrap_or_default();
            let modified = match props.get(0x0E06) {
                Some(PropertyValue::Time(t)) => filetime_to_unix(*t),
                _ => None,
            };
            let title = title(&subject);
            let inner = unique(seen, if here.is_empty() { title } else { format!("{here}{INNER_SEP}{title}") });
            // Extraction (lot 6.7): only the message holding the wanted attachment.
            if capture_done() {
                return;
            }
            if capture_skips("", &inner) {
                continue;
            }
            let names: Vec<String> = attachments.iter().map(|(n, _)| n.clone()).collect();
            let text = message_text(subject.trim(), &sender, "", "", &body, &names);
            out.push(ExtractedDoc { inner: Some(inner.clone()), kind: FileKind::Email, size: Some(text.len() as u64), modified, text });
            for (name, data) in attachments {
                attachment_docs(&name, data, modified, &inner, out);
            }
        }
    }

    if let Some(hierarchy) = folder.hierarchy_table() {
        for row in hierarchy.rows_matrix() {
            let Ok(entry_id) = store.properties().make_entry_id(NodeId::from(u32::from(row.id()))) else { continue };
            let Ok(child) = store.open_folder(&entry_id) else { continue };
            walk(mailbox, &child, Some(&here), depth + 1, out, seen);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eml_message_and_its_attachment() {
        let eml = b"From: Alice <alice@example.com>\r\nTo: bob@example.com\r\nSubject: Devis toiture\r\n\
Date: Fri, 15 Mar 2024 10:30:00 +0000\r\nMIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n\
Voici le devis.\r\n--b\r\nContent-Type: text/plain\r\nContent-Disposition: attachment; filename=\"devis.txt\"\r\n\
Content-Transfer-Encoding: base64\r\n\r\nUmVtcGxhY2VtZW50IGRlcyBnb3V0dGnDqHJlcw==\r\n--b--\r\n";
        let docs = eml_docs(eml).unwrap();
        assert_eq!(docs.len(), 2);
        assert!(docs[0].text.contains("Devis toiture") && docs[0].text.contains("Voici le devis") && docs[0].text.contains("📎 devis.txt"));
        assert_eq!(docs[0].modified, Some(1_710_498_600));
        assert_eq!(docs[1].inner.as_deref(), Some("devis.txt"));
        assert!(docs[1].text.contains("gouttières"), "base64 decoded: {:?}", docs[1].text);
    }

    #[test]
    fn attachment_names_never_break_inner_paths() {
        assert_eq!(clean_name("a/b\\c › d.pdf\0"), "a_b_c - d.pdf");
    }
}
