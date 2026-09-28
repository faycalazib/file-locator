//! Lot 5.8: dates (creation, last access, free range), attributes and
//! digests, through the index and through the live scan, on a folder made
//! for the test in target/tmp.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use prospector_core::disk::sha256_file;
use prospector_core::search::DateField;
use prospector_core::{live_scan, CoreError, Engine, Hit, ScanTarget, SearchRequest, SiteRecord};

struct Fixture {
    root: PathBuf,
    engine: Engine,
    site: SiteRecord,
    _data: tempfile::TempDir,
    _files: tempfile::TempDir,
}

/// plain.txt, lecture.txt (read-only), cache.txt (hidden), copie-a.txt and
/// copie-b.txt (same content), autre.txt (same size, other content).
fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let files = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let root = files.path().to_path_buf();
        let write = |name: &str, text: &str| std::fs::write(root.join(name), text).unwrap();
        write("plain.txt", "Un contrat simple.");
        write("lecture.txt", "Un contrat en lecture seule.");
        write("copie-a.txt", "Le contrat de base, copié.");
        write("copie-b.txt", "Le contrat de base, copié.");
        write("autre.txt", "Le contrat de base, copiés");
        assert_eq!(std::fs::metadata(root.join("autre.txt")).unwrap().len(), std::fs::metadata(root.join("copie-a.txt")).unwrap().len());
        let mut permissions = std::fs::metadata(root.join("lecture.txt")).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(root.join("lecture.txt"), permissions).unwrap();
        hidden_file(&root.join("cache.txt"), "Un contrat caché.");

        let data = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let engine = Engine::open(data.path()).unwrap();
        let site = engine.add_site("Disk", vec![root.to_string_lossy().into_owned()]).unwrap();
        let site = engine.index_site(&site.id, &[], &AtomicBool::new(false), &|_| {}).unwrap();
        Fixture { root, engine, site, _data: data, _files: files }
    })
}

#[cfg(windows)]
fn hidden_file(path: &Path, text: &str) {
    use std::io::Write;
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    let mut file = std::fs::OpenOptions::new().write(true).create(true).truncate(true).attributes(FILE_ATTRIBUTE_HIDDEN).open(path).unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

#[cfg(not(windows))]
fn hidden_file(path: &Path, text: &str) {
    std::fs::write(path.with_file_name(format!(".{}", path.file_name().unwrap().to_string_lossy())), text).unwrap();
}

fn names(hits: &[Hit]) -> Vec<String> {
    let mut out: Vec<String> = hits.iter().map(|h| Path::new(&h.path).file_name().unwrap().to_string_lossy().into_owned()).collect();
    out.sort();
    out
}

fn indexed(request: &SearchRequest) -> Vec<String> {
    let f = fixture();
    names(&f.engine.search(std::slice::from_ref(&f.site.id), request).unwrap().hits)
}

fn scanned(request: &SearchRequest) -> Vec<String> {
    let f = fixture();
    let hits = Mutex::new(Vec::new());
    let target = ScanTarget { site_id: "live".into(), roots: vec![f.root.clone()] };
    live_scan(&[target], &[], request, &AtomicBool::new(false), &|h| hits.lock().unwrap().push(h), &|_, _| {}).unwrap();
    names(&hits.into_inner().unwrap())
}

fn both(request: SearchRequest) -> Vec<String> {
    let from_index = indexed(&request);
    assert_eq!(from_index, scanned(&request), "index and live scan disagree");
    from_index
}

fn contrat() -> SearchRequest {
    SearchRequest { query: "contrat".into(), ..Default::default() }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

const VISIBLE: [&str; 5] = ["autre.txt", "copie-a.txt", "copie-b.txt", "lecture.txt", "plain.txt"];

#[test]
fn creation_and_modification_ranges() {
    assert_eq!(both(contrat()), VISIBLE);
    let hour = 3600;
    for field in [DateField::Created, DateField::Modified] {
        let since = |from: u64| SearchRequest { date_field: field, date_from: Some(from), ..contrat() };
        assert_eq!(both(since(now() - hour)), VISIBLE, "{field:?}: made just now");
        assert!(both(since(now() + hour)).is_empty(), "{field:?}: nothing from the future");
        let until = SearchRequest { date_field: field, date_to: Some(now() - hour), ..contrat() };
        assert!(both(until).is_empty(), "{field:?}: nothing before an hour ago");
    }
}

#[test]
fn last_access_is_read_on_the_disk() {
    let since = |from: u64| SearchRequest { date_field: DateField::Accessed, date_from: Some(from), ..contrat() };
    assert_eq!(both(since(0)), VISIBLE);
    assert!(both(since(now() + 3600)).is_empty());
}

#[test]
fn attributes_read_only_and_hidden() {
    let with = |attrs: &[&str]| SearchRequest { attributes: attrs.iter().map(|a| (*a).to_owned()).collect(), ..contrat() };
    assert_eq!(both(with(&["readOnly"])), ["lecture.txt"]);
    assert!(both(with(&["system"])).is_empty());
    // Hidden files are never indexed; the live scan walks them when asked.
    assert!(indexed(&with(&["hidden"])).is_empty());
    assert_eq!(scanned(&with(&["hidden"])), ["cache.txt"]);
    assert!(!scanned(&contrat()).contains(&"cache.txt".to_owned()), "not without the attribute");
}

#[test]
fn digests_find_the_copies() {
    let f = fixture();
    let original = f.root.join("copie-a.txt");
    let sha = sha256_file(&original).unwrap();
    let size = std::fs::metadata(&original).unwrap().len();
    // "Find copies": digest + exact size, no text.
    let copies = SearchRequest { hash: sha.clone(), min_size: Some(size), max_size: Some(size + 1), ..Default::default() };
    assert_eq!(both(copies), ["copie-a.txt", "copie-b.txt"], "same size, other content: not a copy");
    // Digest alone, upper case, with text.
    assert_eq!(both(SearchRequest { hash: sha.to_uppercase(), ..Default::default() }), ["copie-a.txt", "copie-b.txt"]);
    assert_eq!(both(SearchRequest { hash: sha, ..contrat() }), ["copie-a.txt", "copie-b.txt"]);
    // MD5 of "Un contrat simple." (computed apart, with Python's hashlib).
    let md5 = SearchRequest { hash: "621c122b1081cdfae5b2a24b281c027f".into(), ..Default::default() };
    assert_eq!(both(md5), ["plain.txt"]);
    // Neither MD5 nor SHA-256: refused.
    let invalid = SearchRequest { hash: "1234".into(), ..Default::default() };
    assert!(matches!(f.engine.search(std::slice::from_ref(&f.site.id), &invalid), Err(CoreError::InvalidHash { .. })));
}
