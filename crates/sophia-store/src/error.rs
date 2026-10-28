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
    /// `replace_files`: a record of the file that no name, span or generic
    /// membership reaches, directly or through references. Its manifest would
    /// not reach it, so collection could never remove it (RQ-002).
    #[error("file {path:?}: record {id} is not reached from a name, span or generic membership")]
    UnreachedRecord { path: String, id: String },
    /// A store-truth call on a derived store, whose files are the truth
    /// (SPEC-010): `replace_files_checked`, `propose_files`, `give_truth`.
    #[error("a derived folder store: the files are the truth")]
    Derived,
    /// `replace_files_checked`: files changed since their tokens were read,
    /// as (path, expected, found); nothing was written (SPEC-010 §2).
    #[error("files changed since they were read: {}", conflicts.iter().map(|(p, _, _)| p.as_str()).collect::<Vec<_>>().join(", "))]
    FileConflict { conflicts: Vec<(String, u64, u64)> },
    /// An interface edit of a name that a file lays out, in a store-truth
    /// folder: edit the file (`path`, current `token`) instead (SPEC-010 §2).
    #[error("the name is laid out in file {path:?}: edit it through the file")]
    LaidOut { path: String, token: u64 },
    /// `give_truth`: a file's stored text is not what was exported, or a
    /// file was not exported (SPEC-010 §3).
    #[error("file {path:?} was not exported as stored")]
    ExportMismatch { path: String },
    /// `give_truth`: a name that no current file holds (a definition
    /// written through the interface while the store was the truth). A
    /// fresh build of the files would not have it (SPEC-010 §3).
    #[error("name {name:?} is in no file: export or remove it first")]
    NotInFiles { name: String },
}
