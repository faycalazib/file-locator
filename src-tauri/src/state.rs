//! Shared app state: the engine (replaceable when the data folder moves) and
//! the cancel flags of running indexings, the folder watchers.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};

use prospector_core::Engine;

use crate::watch::Watchers;

pub struct AppState {
    engine: RwLock<Arc<Engine>>,
    pub cancels: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    pub settings_path: PathBuf,
    pub default_data_dir: PathBuf,
    /// Saved searches, alerts, groups: this PC's (lot 7.3).
    pub personal_dir: PathBuf,
    pub watchers: Watchers,
    /// The update plugin is configured (public key + server) in this build.
    pub updates_enabled: bool,
    /// Portable mode (lot 6.8): the data folder next to the program.
    pub portable: Option<PathBuf>,
    /// Page of the latest version (portable mode: downloaded by hand).
    pub releases_page: Option<String>,
}

impl AppState {
    pub fn new(engine: Engine, settings_path: PathBuf, default_data_dir: PathBuf, updates_enabled: bool) -> Self {
        Self {
            personal_dir: engine.personal_dir().to_path_buf(),
            engine: RwLock::new(Arc::new(engine)),
            cancels: Arc::new(Mutex::new(HashMap::new())),
            settings_path,
            default_data_dir,
            watchers: Watchers::default(),
            updates_enabled,
            portable: None,
            releases_page: None,
        }
    }

    pub fn engine(&self) -> Arc<Engine> {
        self.engine.read().map(|e| e.clone()).unwrap_or_else(|p| p.into_inner().clone())
    }

    pub fn replace_engine(&self, engine: Engine) {
        if let Ok(mut slot) = self.engine.write() {
            *slot = Arc::new(engine);
        }
    }

    /// Stops every running indexing (shared index taken over by another PC).
    pub fn cancel_all(&self) {
        if let Ok(map) = self.cancels.lock() {
            for flag in map.values() {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    pub fn set_cancel(&self, id: &str, flag: Arc<AtomicBool>) {
        if let Ok(mut map) = self.cancels.lock() {
            map.insert(id.to_owned(), flag);
        }
    }

    pub fn cancel_flag(&self, id: &str) -> Option<Arc<AtomicBool>> {
        self.cancels.lock().ok().and_then(|map| map.get(id).cloned())
    }
}
