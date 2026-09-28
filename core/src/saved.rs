//! Saved searches (goal.md §4): kept next to the indexes (`saved-searches.json`),
//! so they follow the data folder when it moves.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};
use crate::fsutil::write_atomic;
use crate::search::SearchRequest;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSearch {
    pub id: String,
    /// Chosen by the user: data, never translated.
    pub label: String,
    pub query: String,
    /// Unix seconds.
    pub created: u64,
    /// Mode, options, filters and sites of the search. Their shape belongs to
    /// the UI (relative date filters, presets…): stored as is.
    #[serde(default)]
    pub settings: serde_json::Value,
    /// "Alert me" (lot 6.1); absent when off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alert: Option<Alert>,
}

/// An alert on a saved search: the files that start matching are announced.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    /// Built by the UI from the saved settings (relative dates left out).
    pub request: SearchRequest,
    pub site_ids: Vec<String>,
    /// Documents already matching (never announced again).
    #[serde(default)]
    pub seen: BTreeSet<String>,
    /// Announced, not looked at yet (the badge in the rail).
    #[serde(default)]
    pub fresh: Vec<String>,
}

impl PartialEq for SearchRequest {
    fn eq(&self, other: &Self) -> bool {
        serde_json::to_value(self).ok() == serde_json::to_value(other).ok()
    }
}

pub struct SavedSearches {
    file: PathBuf,
    list: Mutex<Vec<SavedSearch>>,
}

impl SavedSearches {
    pub fn open(data_dir: &Path) -> Result<Self> {
        let file = data_dir.join("saved-searches.json");
        let list = if file.exists() { serde_json::from_slice(&std::fs::read(&file)?)? } else { Vec::new() };
        Ok(Self { file, list: Mutex::new(list) })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<SavedSearch>> {
        self.list.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn write(&self, list: &[SavedSearch]) -> Result<()> {
        Ok(write_atomic(&self.file, &serde_json::to_vec_pretty(list)?)?)
    }

    pub fn list(&self) -> Vec<SavedSearch> {
        self.lock().clone()
    }

    /// Saves a search. The same query with the same settings is saved once:
    /// saving it again only renames it.
    pub fn save(&self, label: &str, query: &str, settings: serde_json::Value) -> Result<SavedSearch> {
        let mut list = self.lock();
        let label = if label.trim().is_empty() { query.trim() } else { label.trim() }.to_owned();
        let saved = if let Some(existing) = list.iter_mut().find(|s| s.query == query && s.settings == settings) {
            existing.label = label;
            existing.clone()
        } else {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
            let mut stamp = now.as_millis();
            while list.iter().any(|s| s.id == format!("s{stamp:x}")) {
                stamp += 1;
            }
            let saved = SavedSearch {
                id: format!("s{stamp:x}"),
                label,
                query: query.to_owned(),
                created: now.as_secs(),
                settings,
                alert: None,
            };
            list.push(saved.clone());
            saved
        };
        self.write(&list)?;
        Ok(saved)
    }

    /// Removes a saved search (no error if it is already gone).
    pub fn remove(&self, id: &str) -> Result<()> {
        let mut list = self.lock();
        let before = list.len();
        list.retain(|s| s.id != id);
        if list.len() != before {
            self.write(&list)?;
        }
        Ok(())
    }

    /// Runs `change` on one saved search, then writes the list.
    fn update<T>(&self, id: &str, change: impl FnOnce(&mut SavedSearch) -> T) -> Result<T> {
        let mut list = self.lock();
        let entry = list.iter_mut().find(|s| s.id == id).ok_or_else(|| CoreError::SavedSearchNotFound { id: id.to_owned() })?;
        let out = change(entry);
        self.write(&list)?;
        Ok(out)
    }

    pub fn set_alert(&self, id: &str, alert: Option<Alert>) -> Result<SavedSearch> {
        self.update(id, |s| {
            s.alert = alert;
            s.clone()
        })
    }

    /// The saved searches with an alert on this site.
    pub fn alerts_on(&self, site_id: &str) -> Vec<SavedSearch> {
        self.lock().iter().filter(|s| s.alert.as_ref().is_some_and(|a| a.site_ids.iter().any(|i| i == site_id))).cloned().collect()
    }

    /// Of `found`, the documents never seen by this alert: they are marked
    /// seen and fresh, and returned. Removed documents leave `seen`, so a
    /// file deleted then restored is announced again.
    pub fn record(&self, id: &str, found: &[String], removed_files: &[String]) -> Result<Vec<String>> {
        self.update(id, |s| {
            let Some(alert) = s.alert.as_mut() else { return Vec::new() };
            if !removed_files.is_empty() {
                let gone = |doc: &String| removed_files.iter().any(|f| doc == f || doc.starts_with(&format!("{f}{}", crate::extract::INNER_SEP)));
                alert.seen.retain(|doc| !gone(doc));
                alert.fresh.retain(|doc| !gone(doc));
            }
            let new: Vec<String> = found.iter().filter(|doc| alert.seen.insert((*doc).clone())).cloned().collect();
            alert.fresh.extend(new.iter().cloned());
            new
        })
    }

    /// The user looked at the results: no more badge.
    pub fn mark_read(&self, id: &str) -> Result<SavedSearch> {
        self.update(id, |s| {
            if let Some(alert) = s.alert.as_mut() {
                alert.fresh.clear();
            }
            s.clone()
        })
    }
}
