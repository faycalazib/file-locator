//! Errors of the engine. They cross the Tauri bridge as `{ code, params }`:
//! the UI translates the code (goal.md §5bis-A, never translated text here).

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "code", content = "params", rename_all = "camelCase")]
pub enum CoreError {
    #[error("folder not found: {path}")]
    RootUnavailable { path: String },

    #[error("dig site not found: {id}")]
    SiteNotFound { id: String },

    #[error("file not found in the index: {path}")]
    FileNotIndexed { path: String },

    #[error("index is busy: {id}")]
    IndexBusy { id: String },

    #[error("index error: {message}")]
    Index { message: String },

    #[error("storage error: {message}")]
    Storage { message: String },

    #[error("invalid query: {message}")]
    InvalidQuery { message: String },

    #[error("saved search not found: {id}")]
    SavedSearchNotFound { id: String },

    /// Neither an MD5 (32 hex digits) nor a SHA-256 (64).
    #[error("invalid digest: {value}")]
    InvalidHash { value: String },

    /// A document inside an archive or an e-mail that cannot be extracted
    /// (a message, not a file; the container changed or is unreadable).
    #[error("cannot extract: {path}")]
    NotExtractable { path: String },

    /// A site group without a name or without sites (lot 7.2).
    #[error("a group needs a name and at least one site")]
    InvalidGroup,

    #[error("site group not found: {id}")]
    GroupNotFound { id: String },

    /// Shared index (lot 7.3): another PC keeps it up to date.
    #[error("{pc} keeps this shared index up to date")]
    ReadOnly { pc: String },

    /// Meaning search (Étape 8): module missing, damaged or unusable.
    #[error("meaning search: {message}")]
    Sense { message: String },
}

impl From<tantivy::TantivyError> for CoreError {
    fn from(e: tantivy::TantivyError) -> Self {
        match e {
            tantivy::TantivyError::LockFailure(_, _) => CoreError::IndexBusy { id: String::new() },
            other => CoreError::Index { message: other.to_string() },
        }
    }
}

impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        CoreError::Storage { message: e.to_string() }
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(e: serde_json::Error) -> Self {
        CoreError::Storage { message: e.to_string() }
    }
}

impl From<tantivy::directory::error::OpenDirectoryError> for CoreError {
    fn from(e: tantivy::directory::error::OpenDirectoryError) -> Self {
        CoreError::Storage { message: e.to_string() }
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;
