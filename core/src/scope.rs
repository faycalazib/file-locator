//! "Search in this folder" (lot 5.7, Explorer right-click): a search limited
//! to one folder, whether it lies inside a dig site or not.
//!
//! Windows paths ignore case and accept `/` as well as `\`: the folder given
//! by the Explorer is compared to the sites' roots that way, then rewritten
//! with the root's own spelling, which is the one stored in the index.

use std::path::{PathBuf, MAIN_SEPARATOR};

use crate::scan::ScanTarget;

/// Site id of the hits of a folder that no dig site holds (live scan only).
pub const FOLDER_SITE: &str = "@folder";

fn is_sep(c: char) -> bool {
    c == '\\' || c == '/'
}

/// One character, case folded without changing the number of characters.
fn fold(c: char) -> char {
    if is_sep(c) {
        return '\\';
    }
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

fn trim_seps(path: &str) -> &str {
    path.trim_end_matches(is_sep)
}

/// The part of `path` after `root` when `path` is `root` itself or lies
/// inside it (`""` or `\Dupont\2024`); `None` otherwise. Unlike
/// `manifest::is_within` (paths read from the disk, spelled the same), case
/// is ignored: the folder comes from the Explorer or the command line.
pub fn inside<'a>(root: &str, path: &'a str) -> Option<&'a str> {
    let root = trim_seps(root);
    let path = trim_seps(path);
    let mut rest = path.char_indices();
    for r in root.chars() {
        let (_, p) = rest.next()?;
        if fold(p) != fold(r) {
            return None;
        }
    }
    let tail = rest.next().map_or("", |(at, _)| &path[at..]);
    (tail.is_empty() || tail.starts_with(is_sep)).then_some(tail)
}

/// `folder` spelled as on the disk, from `root` (which holds it): the index
/// stores paths with that spelling. `D:\clients\DUPONT` under `D:\Clients`
/// gives `D:\Clients\Dupont` (each folder name as its parent lists it; a
/// name that cannot be listed is kept as given).
fn respell(root: &str, tail: &str) -> String {
    let mut path = trim_seps(root).to_owned();
    for part in tail.split(is_sep).filter(|p| !p.is_empty()) {
        let listed = std::fs::read_dir(format!("{path}{MAIN_SEPARATOR}")).ok().and_then(|entries| {
            entries.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).find(|name| same_name(name, part))
        });
        path.push(MAIN_SEPARATOR);
        path.push_str(listed.as_deref().unwrap_or(part));
    }
    path
}

fn same_name(a: &str, b: &str) -> bool {
    a.chars().count() == b.chars().count() && a.chars().zip(b.chars()).all(|(x, y)| fold(x) == fold(y))
}

/// Where a search limited to `folder` must look, among the sites' targets:
/// - a site holds the folder: that folder only, labelled with the site (its
///   documents keep their site, the preview reads them from the index);
/// - otherwise the folder itself, labelled [`FOLDER_SITE`]: sites inside it
///   are covered by that walk.
pub fn restrict_targets(targets: &[ScanTarget], folder: &str) -> Vec<ScanTarget> {
    for target in targets {
        for root in &target.roots {
            let root = root.to_string_lossy();
            if let Some(tail) = inside(&root, folder) {
                return vec![ScanTarget { site_id: target.site_id.clone(), roots: vec![PathBuf::from(respell(&root, tail))] }];
            }
        }
    }
    vec![ScanTarget { site_id: FOLDER_SITE.to_owned(), roots: vec![PathBuf::from(folder)] }]
}

/// How one indexed site answers a search limited to `folder`.
#[derive(Debug, PartialEq, Eq)]
pub enum SiteScope {
    /// The folder covers the whole site (or no folder is asked).
    Whole,
    /// Only the documents whose path starts with one of these prefixes
    /// (separator included, spelled as in the index).
    Prefixes(Vec<String>),
    /// The site has nothing in the folder.
    Outside,
}

pub fn site_scope(roots: &[String], folder: Option<&str>) -> SiteScope {
    let Some(folder) = folder else {
        return SiteScope::Whole;
    };
    if let Some((root, tail)) = roots.iter().find_map(|r| inside(r, folder).map(|t| (r, t))) {
        return if tail.is_empty() { SiteScope::Whole } else { SiteScope::Prefixes(vec![prefix(&respell(root, tail))]) };
    }
    // Roots inside the folder: all of them = the whole site, else those ones.
    let within: Vec<&String> = roots.iter().filter(|r| inside(folder, r).is_some()).collect();
    match within.len() {
        0 => SiteScope::Outside,
        n if n == roots.len() => SiteScope::Whole,
        _ => SiteScope::Prefixes(within.into_iter().map(|r| prefix(r)).collect()),
    }
}

/// `D:\Clients` → `D:\Clients\`: only what is inside, not `D:\Clients2`.
fn prefix(folder: &str) -> String {
    format!("{}{MAIN_SEPARATOR}", trim_seps(folder))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_ignores_case_and_separators() {
        assert_eq!(inside(r"D:\Clients", r"d:\clients\Dupont"), Some(r"\Dupont"));
        assert_eq!(inside(r"D:\Clients\", r"D:/Clients/Dupont/"), Some("/Dupont"));
        assert_eq!(inside(r"D:\Clients", r"D:\Clients"), Some(""));
        assert_eq!(inside(r"D:\", r"D:\Clients"), Some(r"\Clients"));
        assert_eq!(inside(r"D:\Clients", r"D:\Clients2"), None, "a longer name is another folder");
        assert_eq!(inside(r"D:\Clients\Dupont", r"D:\Clients"), None);
        assert_eq!(inside(r"D:\Élèves", r"d:\éLÈVES\2024"), Some(r"\2024"));
    }

    #[test]
    fn a_folder_inside_a_site_keeps_the_site_and_its_spelling() {
        let targets = [
            ScanTarget { site_id: "code".into(), roots: vec![PathBuf::from(r"E:\Code")] },
            ScanTarget { site_id: "clients".into(), roots: vec![PathBuf::from(r"D:\Clients")] },
        ];
        let restricted = restrict_targets(&targets, r"d:\clients\Dupont");
        assert_eq!(restricted.len(), 1);
        assert_eq!(restricted[0].site_id, "clients");
        assert_eq!(restricted[0].roots, vec![PathBuf::from(r"D:\Clients\Dupont")]);
    }

    #[test]
    fn a_folder_outside_every_site_is_walked_on_its_own() {
        let targets = [ScanTarget { site_id: "clients".into(), roots: vec![PathBuf::from(r"D:\Clients")] }];
        let restricted = restrict_targets(&targets, r"D:\");
        assert_eq!(restricted[0].site_id, FOLDER_SITE);
        assert_eq!(restricted[0].roots, vec![PathBuf::from(r"D:\")]);
    }

    #[test]
    fn site_scope_by_case() {
        let roots = vec![r"D:\Clients".to_owned()];
        assert_eq!(site_scope(&roots, None), SiteScope::Whole);
        assert_eq!(site_scope(&roots, Some(r"d:\clients")), SiteScope::Whole);
        assert_eq!(site_scope(&roots, Some(r"D:\")), SiteScope::Whole);
        assert_eq!(site_scope(&roots, Some(r"d:\clients\dupont")), SiteScope::Prefixes(vec![format!(r"D:\Clients\dupont{MAIN_SEPARATOR}")]));
        assert_eq!(site_scope(&roots, Some(r"E:\Code")), SiteScope::Outside);
        let two = vec![r"D:\Clients".to_owned(), r"E:\Code".to_owned()];
        assert_eq!(site_scope(&two, Some(r"D:\")), SiteScope::Prefixes(vec![format!(r"D:\Clients{MAIN_SEPARATOR}")]));
    }
}
