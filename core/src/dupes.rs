//! Duplicates (lot 6.5).
//! - Identical files: grouped by size first (from the manifests: no walk),
//!   then only the files sharing a size are hashed (SHA-256).
//! - Near-identical documents: MinHash signatures of their indexed text
//!   (5-word shingles, accents and case ignored), candidate pairs by LSH
//!   bands, kept above a similarity threshold, then grouped.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// What to look for.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DupOptions {
    pub exact: bool,
    pub similar: bool,
    /// Minimum similarity of near-identical documents, 0.5 to 1.
    pub threshold: f32,
}

impl Default for DupOptions {
    fn default() -> Self {
        Self { exact: true, similar: true, threshold: 0.8 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupFile {
    pub site_id: String,
    /// A file, or `file › inner` for a document inside it.
    pub path: String,
    pub size: u64,
    /// Unix seconds.
    pub modified: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DupKind {
    Exact,
    Similar,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupGroup {
    pub kind: DupKind,
    /// 1 for identical files; the lowest similarity of the group otherwise.
    pub similarity: f32,
    pub files: Vec<DupFile>,
    /// Space taken by the copies beyond the first (identical files only).
    pub wasted: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupReport {
    pub groups: Vec<DupGroup>,
    pub files_hashed: usize,
    pub docs_compared: usize,
    pub took_ms: u64,
    pub cancelled: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DupPhase {
    Hashing,
    Comparing,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupProgress {
    pub phase: DupPhase,
    pub done: usize,
    pub total: usize,
}

/// Identical files among `files` (site, path, size, modified): only the
/// ones sharing a size are hashed, in path order (kind to hard disks).
pub fn exact_groups(files: Vec<DupFile>, cancel: &AtomicBool, progress: &dyn Fn(usize, usize)) -> (Vec<DupGroup>, usize) {
    let mut by_size: BTreeMap<u64, Vec<DupFile>> = BTreeMap::new();
    for file in files.into_iter().filter(|f| f.size > 0) {
        by_size.entry(file.size).or_default().push(file);
    }
    let mut candidates: Vec<DupFile> = by_size.into_values().filter(|group| group.len() > 1).flatten().collect();
    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    let total = candidates.len();
    let mut by_hash: HashMap<String, Vec<DupFile>> = HashMap::new();
    for (i, file) in candidates.into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if let Ok(hash) = crate::disk::sha256_file(std::path::Path::new(&file.path)) {
            by_hash.entry(hash).or_default().push(file);
        }
        if (i + 1).is_multiple_of(16) || i + 1 == total {
            progress(i + 1, total);
        }
    }
    let groups = by_hash
        .into_values()
        .filter(|g| g.len() > 1)
        .map(|files| {
            let wasted = files[0].size * (files.len() as u64 - 1);
            DupGroup { kind: DupKind::Exact, similarity: 1.0, files, wasted }
        })
        .collect();
    (groups, total)
}

/// Documents shorter than this are not compared (too little text to judge).
pub const MIN_CHARS: usize = 300;
const SHINGLE: usize = 5;
const HASHES: usize = 64;
const BANDS: usize = 16;
const ROWS: usize = HASHES / BANDS;
/// Text looked at per document.
const MAX_CHARS: usize = 200_000;
/// A bucket holding more documents than this is compared as a chain, not
/// pair by pair (the same boilerplate everywhere).
const MAX_BUCKET: usize = 200;

/// `(multiplier, offset)` pairs: one cheap hash function per signature slot.
fn coefficients() -> [(u64, u64); HASHES] {
    let mut out = [(0, 0); HASHES];
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    for slot in &mut out {
        // SplitMix64.
        let mut next = || {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        *slot = (next() | 1, next());
    }
    out
}

/// MinHash signature of a text: `None` when it is too short to compare.
pub fn signature(text: &str) -> Option<[u32; HASHES]> {
    if text.chars().count() < MIN_CHARS {
        return None;
    }
    let cut = text.char_indices().nth(MAX_CHARS).map_or(text.len(), |(i, _)| i);
    let folded = crate::lang::fold(&text[..cut]);
    let words: Vec<&str> = folded.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    if words.len() < SHINGLE {
        return None;
    }
    let coefficients = coefficients();
    let mut sig = [u32::MAX; HASHES];
    for window in words.windows(SHINGLE) {
        let mut hasher = DefaultHasher::new();
        window.hash(&mut hasher);
        let h = hasher.finish();
        for (slot, (a, b)) in sig.iter_mut().zip(coefficients.iter()) {
            let v = (h.wrapping_mul(*a).wrapping_add(*b) >> 32) as u32;
            if v < *slot {
                *slot = v;
            }
        }
    }
    Some(sig)
}

fn similarity(a: &[u32; HASHES], b: &[u32; HASHES]) -> f32 {
    a.iter().zip(b).filter(|(x, y)| x == y).count() as f32 / HASHES as f32
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// Near-identical documents. `docs` = (file, its text); `skip(a, b)` leaves
/// out pairs already known (identical files, the same document).
pub fn similar_groups(
    docs: Vec<(DupFile, String)>,
    threshold: f32,
    skip: &(dyn Fn(&DupFile, &DupFile) -> bool + Sync),
    cancel: &AtomicBool,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> (Vec<DupGroup>, usize) {
    let total = docs.len();
    let done = std::sync::atomic::AtomicUsize::new(0);
    let signed: Vec<(DupFile, [u32; HASHES])> = docs
        .into_par_iter()
        .filter_map(|(file, text)| {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            let sig = signature(&text);
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n.is_multiple_of(256) || n == total {
                progress(n, total);
            }
            sig.map(|s| (file, s))
        })
        .collect();

    // LSH: documents sharing a whole band are candidates.
    let mut buckets: HashMap<(usize, [u32; ROWS]), Vec<usize>> = HashMap::new();
    for (i, (_, sig)) in signed.iter().enumerate() {
        for band in 0..BANDS {
            let mut key = [0u32; ROWS];
            key.copy_from_slice(&sig[band * ROWS..(band + 1) * ROWS]);
            buckets.entry((band, key)).or_default().push(i);
        }
    }
    let mut pairs: HashSet<(usize, usize)> = HashSet::new();
    for members in buckets.values().filter(|m| m.len() > 1) {
        if members.len() > MAX_BUCKET {
            pairs.extend(members.windows(2).map(|w| (w[0].min(w[1]), w[0].max(w[1]))));
        } else {
            for (x, &a) in members.iter().enumerate() {
                for &b in &members[x + 1..] {
                    pairs.insert((a.min(b), a.max(b)));
                }
            }
        }
    }

    let mut parent: Vec<usize> = (0..signed.len()).collect();
    let mut lowest: HashMap<usize, f32> = HashMap::new();
    let mut edges = Vec::new();
    for (a, b) in pairs {
        let (fa, sa) = &signed[a];
        let (fb, sb) = &signed[b];
        if fa.path == fb.path || skip(fa, fb) {
            continue;
        }
        let s = similarity(sa, sb);
        if s >= threshold {
            edges.push((a, b, s));
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            if ra != rb {
                parent[ra] = rb;
            }
        }
    }
    for (a, _, s) in edges {
        let root = find(&mut parent, a);
        let entry = lowest.entry(root).or_insert(1.0);
        *entry = entry.min(s);
    }
    let mut members: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..signed.len() {
        let root = find(&mut parent, i);
        if lowest.contains_key(&root) {
            members.entry(root).or_default().push(i);
        }
    }
    let groups = members
        .into_iter()
        .map(|(root, list)| DupGroup {
            kind: DupKind::Similar,
            similarity: lowest[&root],
            files: list.into_iter().map(|i| signed[i].0.clone()).collect(),
            wasted: 0,
        })
        .collect();
    (groups, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 120 words of a vocabulary of its own (`prefix`).
    fn text(prefix: &str, step: usize) -> String {
        (0..120).map(|i| format!("{prefix}{} ", (i * step + 3) % 97)).collect()
    }

    #[test]
    fn a_retouched_text_is_close_a_different_one_is_not() {
        let base = format!("Contrat de prestation entre les parties. {}", text("mot", 7));
        let retouched = base.replacen("mot7 ", "mot7 modifié ", 1);
        let other = format!("Rapport annuel sans rapport. {}", text("terme", 11));
        let (a, b, c) = (signature(&base).unwrap(), signature(&retouched).unwrap(), signature(&other).unwrap());
        assert!(similarity(&a, &b) >= 0.8, "{}", similarity(&a, &b));
        assert!(similarity(&a, &c) < 0.5, "{}", similarity(&a, &c));
        assert!(signature("trop court").is_none());
    }
}
