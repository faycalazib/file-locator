//! Site groups (lot 7.2): named sets of dig sites ("Clients", "Code"),
//! applied in one click or with Ctrl+1…9, and `--group` on the command line.
//! Kept next to the indexes (`site-groups.json`), so they follow the data
//! folder (and the portable drive).

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};
use crate::fsutil::write_atomic;

/// Pencil colors the UI gives to groups, in this order.
pub const COLORS: usize = 6;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteGroup {
    pub id: String,
    /// Chosen by the user: data, never translated.
    pub name: String,
    pub site_ids: Vec<String>,
    /// Index of its color (0 … COLORS-1), chosen at creation.
    pub color: usize,
}

pub struct SiteGroups {
    file: PathBuf,
    list: Mutex<Vec<SiteGroup>>,
}

impl SiteGroups {
    pub fn open(data_dir: &Path) -> Result<Self> {
        let file = data_dir.join("site-groups.json");
        let list = if file.exists() { serde_json::from_slice(&std::fs::read(&file)?)? } else { Vec::new() };
        Ok(Self { file, list: Mutex::new(list) })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<SiteGroup>> {
        self.list.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn write(&self, list: &[SiteGroup]) -> Result<()> {
        Ok(write_atomic(&self.file, &serde_json::to_vec_pretty(list)?)?)
    }

    /// In creation order: the n-th group answers to Ctrl+n.
    pub fn list(&self) -> Vec<SiteGroup> {
        self.lock().clone()
    }

    /// A new group; its color is the first one no other group has.
    pub fn create(&self, name: &str, site_ids: Vec<String>) -> Result<SiteGroup> {
        let name = name.trim();
        if name.is_empty() || site_ids.is_empty() {
            return Err(CoreError::InvalidGroup);
        }
        let mut list = self.lock();
        let mut stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        while list.iter().any(|g| g.id == format!("g{stamp:x}")) {
            stamp += 1;
        }
        let color = (0..COLORS).find(|c| !list.iter().any(|g| g.color == *c)).unwrap_or(list.len() % COLORS);
        let group = SiteGroup { id: format!("g{stamp:x}"), name: name.to_owned(), site_ids: dedup(site_ids), color };
        list.push(group.clone());
        self.write(&list)?;
        Ok(group)
    }

    /// Renames a group and / or changes its sites (an empty list is refused:
    /// remove the group instead).
    pub fn update(&self, id: &str, name: Option<&str>, site_ids: Option<Vec<String>>) -> Result<SiteGroup> {
        let mut list = self.lock();
        let group = list.iter_mut().find(|g| g.id == id).ok_or_else(|| CoreError::GroupNotFound { id: id.to_owned() })?;
        if let Some(name) = name.map(str::trim) {
            if name.is_empty() {
                return Err(CoreError::InvalidGroup);
            }
            group.name = name.to_owned();
        }
        if let Some(site_ids) = site_ids {
            if site_ids.is_empty() {
                return Err(CoreError::InvalidGroup);
            }
            group.site_ids = dedup(site_ids);
        }
        let group = group.clone();
        self.write(&list)?;
        Ok(group)
    }

    /// Removes a group (no error if it is already gone).
    pub fn remove(&self, id: &str) -> Result<()> {
        let mut list = self.lock();
        let before = list.len();
        list.retain(|g| g.id != id);
        if list.len() != before {
            self.write(&list)?;
        }
        Ok(())
    }

    /// A site was removed: it leaves its groups; a group left empty goes too.
    pub fn forget_site(&self, site_id: &str) -> Result<()> {
        let mut list = self.lock();
        let before = list.clone();
        for group in list.iter_mut() {
            group.site_ids.retain(|s| s != site_id);
        }
        list.retain(|g| !g.site_ids.is_empty());
        if *list != before {
            self.write(&list)?;
        }
        Ok(())
    }
}

fn dedup(ids: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(ids.len());
    for id in ids {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn create_rename_forget_and_reload() {
        let dir = crate::test_tmp();
        let groups = SiteGroups::open(dir.path()).unwrap();
        let clients = groups.create(" Clients ", ids(&["a", "b", "a"])).unwrap();
        assert_eq!((clients.name.as_str(), clients.site_ids.clone(), clients.color), ("Clients", ids(&["a", "b"]), 0));
        let code = groups.create("Code", ids(&["c"])).unwrap();
        assert_eq!(code.color, 1);
        assert!(groups.create("", ids(&["a"])).is_err());
        assert!(groups.create("Vide", Vec::new()).is_err());

        groups.update(&clients.id, Some("Clients & RH"), None).unwrap();
        assert!(groups.update(&clients.id, None, Some(Vec::new())).is_err());
        assert!(groups.update("absent", Some("x"), None).is_err());

        // A removed site leaves its groups; "Code" is left empty and goes.
        groups.forget_site("c").unwrap();
        groups.forget_site("a").unwrap();
        let again = SiteGroups::open(dir.path()).unwrap().list();
        assert_eq!(again.len(), 1);
        assert_eq!((again[0].name.as_str(), again[0].site_ids.clone()), ("Clients & RH", ids(&["b"])));

        // The color of a removed group is given again.
        let next = groups.create("Archives", ids(&["b"])).unwrap();
        assert_eq!(next.color, 1);
        groups.remove(&next.id).unwrap();
        groups.remove(&next.id).unwrap();
        assert_eq!(groups.list().len(), 1);
    }
}
