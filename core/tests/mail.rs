//! Lot 5.4: e-mail attachments (.msg, .eml) and mbox mailboxes, on
//! test_fixtures/mail, through the index and through the live scan.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, OnceLock};

use prospector_core::{live_scan, Engine, Hit, ScanTarget, SearchRequest, SiteRecord};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("test_fixtures").canonicalize().unwrap()
}

fn roots() -> Vec<PathBuf> {
    vec![fixtures().join("mail")]
}

fn engine() -> &'static (Engine, SiteRecord, tempfile::TempDir) {
    static ENGINE: OnceLock<(Engine, SiteRecord, tempfile::TempDir)> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let engine = Engine::open(data.path()).unwrap();
        let site = engine.add_site("Mail", roots().iter().map(|r| r.to_string_lossy().into_owned()).collect()).unwrap();
        let site = engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
        (engine, site, data)
    })
}

fn rels(hits: &[Hit]) -> Vec<String> {
    let root = fixtures().to_string_lossy().into_owned();
    let mut out: Vec<String> = hits
        .iter()
        .map(|h| h.path.strip_prefix(&root).unwrap_or(&h.path).trim_start_matches(['\\', '/']).replace('\\', "/"))
        .collect();
    out.sort();
    out
}

fn indexed(request: &SearchRequest) -> Vec<Hit> {
    let (engine, site, _) = engine();
    engine.search(std::slice::from_ref(&site.id), request).unwrap().hits
}

fn both(request: SearchRequest) -> Vec<String> {
    let from_index = rels(&indexed(&request));
    let hits = Mutex::new(Vec::new());
    let targets = [ScanTarget { site_id: "live".into(), roots: roots() }];
    live_scan(&targets, &[], &request, &AtomicBool::new(false), &|h| hits.lock().unwrap().push(h), &|_, _| {}).unwrap();
    assert_eq!(from_index, rels(&hits.into_inner().unwrap()), "index and live scan disagree");
    from_index
}

fn text(query: &str) -> SearchRequest {
    SearchRequest { query: query.into(), ..Default::default() }
}

#[test]
fn attachments_of_an_eml_including_an_attached_zip() {
    assert_eq!(both(text("chevronnage")), ["mail/devis-charpente.eml › devis-charpente.docx"]);
    assert_eq!(both(text("voliges")), ["mail/devis-charpente.eml › plans.zip › plans/notice.txt"]);
    let hit = &indexed(&text("chevronnage"))[0];
    assert_eq!(hit.kind, "email", "an attachment belongs to the e-mail family");
    assert_eq!(hit.modified, 1_710_498_600, "dated like its message");
    // The message itself lists its attachments.
    assert_eq!(both(text("\"devis-charpente.docx\"")), ["mail/devis-charpente.eml"]);
}

#[test]
fn attachment_of_a_msg_read_with_its_own_format() {
    // A Word 97 .doc attached to an Outlook message (msg_parser sample).
    assert!(both(text("phishing")).contains(&"mail/avec-piece-jointe.msg › loan_proposal.doc".to_owned()));
}

#[test]
fn mbox_messages_with_the_same_subject_and_their_attachments() {
    assert_eq!(
        both(text("livraison")),
        ["mail/fournisseur.mbox › Commande de tuiles", "mail/fournisseur.mbox › Commande de tuiles (2) › bon.txt"]
    );
    assert_eq!(both(text("faîtage")), ["mail/fournisseur.mbox › Commande de tuiles (2) › bon.txt"]);
}

#[test]
fn attachments_answer_to_the_name_criterion() {
    let docx = SearchRequest { name_pattern: "*.docx".into(), ..Default::default() };
    assert_eq!(both(docx), ["mail/devis-charpente.eml › devis-charpente.docx"]);
}
