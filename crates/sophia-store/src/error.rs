//! Store errors. See `error.md`.

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("backend error: {0}")]
    Backend(String),
    #[error("stored value could not be decoded: {0}")]
    Codec(String),
    #[error("name {0:?} already exists")]
    NameExists(String),
    #[error("no name {0:?}")]
    NoSuchName(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("name {path:?} changed: expected version {expected}, found {found}")]
    Conflict { path: String, expected: u64, found: u64 },
    #[error("invalid name path {path:?}: segment {segment:?}")]
    InvalidPath { path: String, segment: String },
    #[error("query refused: {0}")]
    QueryRefused(String),
    /// A record arrived with an id already stored under another kind. A
    /// kind must follow from content (SP-012), so this is a frontend bug.
    #[error("record {id} is stored with kind {stored:?}, not {new:?}")]
    KindConflict { id: String, stored: String, new: String },
}
