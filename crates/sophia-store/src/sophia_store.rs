//! The Sophia store: content-addressed definitions of any language, names,
//! nominal entities and their versioned relations, lineages, annotations,
//! derived facts and the change log, over interchangeable backends.
//! See `sophia_store.md` and SPEC-001.

pub mod backend;
pub mod codec;
pub mod query;
mod error;
pub mod folder;
pub mod logic;
pub mod types;

#[cfg(any(feature = "redb", feature = "lmdb"))]
pub mod kv;
#[cfg(feature = "redb")]
pub mod redb_store;
#[cfg(feature = "lmdb")]
pub mod lmdb_store;
#[cfg(feature = "duckdb")]
pub mod duckdb_store;
#[cfg(feature = "sqlite")]
pub mod sqlite;

pub use backend::{Backend, ReadTx, WriteTx};
pub use error::StoreError;
pub use logic::{AddReport, Store, UpdateReport, validate_path};
pub use types::*;
pub use folder::{FileGeneric, FileIngest, FileManifest, FileReport, TextSource};
