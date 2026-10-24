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
    /// `replace_files`: a name is owned by a file outside the batch (its
    /// path in `owner`; empty when no file owns it). Retry with that file
    /// in the batch (SPEC-009 §7).
    #[error("name {name:?} is claimed by {}", if owner.is_empty() { "something that is not a file of this batch".to_string() } else { format!("file {owner:?}") })]
    NameClaimed { name: String, owner: String },
    /// `replace_files`: the folder was built with another normaliser; it
    /// must be rebuilt (SPEC-009 §2).
    #[error("the folder store was built with normaliser {stored:?}, not {new:?}: rebuild it")]
    NormaliserChanged { stored: String, new: String },
    /// `collect_folder`: only a derived store, whose files are the truth,
    /// is collected; elsewhere the store keeps its history (SR-146).
    #[error("not a derived folder store: the store is the source of truth and keeps its history")]
    NotDerived,
}
