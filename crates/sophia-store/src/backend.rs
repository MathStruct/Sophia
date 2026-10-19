//! The backend interface: typed operations inside transactions (SPEC-001
//! §12). The store's logic ([`crate::Store`]) is written once against these
//! traits; each backend implements them over its own tables. See
//! `backend.md`.

use sophia_hash::{Digest, NominalId, Record};

use crate::error::StoreError;
use crate::types::*;

pub type Result<T> = std::result::Result<T, StoreError>;

/// Reads, inside a transaction (a consistent snapshot).
pub trait ReadTx {
    fn record(&self, id: &Digest) -> Result<Option<Record>>;
    fn has_record(&self, id: &Digest) -> Result<bool>;
    /// The members of a component, in canonical index order.
    fn component(&self, component: &Digest) -> Result<Vec<Record>>;
    /// Definitions that reference `id` directly.
    fn dependants(&self, id: &Digest) -> Result<Vec<Digest>>;
    fn record_count(&self) -> Result<u64>;
    /// Ids of all records of a kind (or every kind), up to `limit`.
    fn records_of_kind(&self, kind: Option<&str>, limit: usize) -> Result<Vec<Digest>>;

    fn name(&self, path: &str) -> Result<Option<NameEntry>>;
    /// Names at or below `prefix` (a dotted path, or "" for all), sorted, up to `limit`.
    fn names_under(&self, prefix: &str, limit: usize) -> Result<Vec<NameEntry>>;
    fn names_of(&self, target: &Digest) -> Result<Vec<NameEntry>>;

    fn last_seq(&self) -> Result<u64>;
    fn log_since(&self, seq: u64, limit: usize) -> Result<Vec<LogEntry>>;

    fn nominal(&self, id: &NominalId) -> Result<Option<Nominal>>;
    /// Current members (not removed) of a nominal entity's relation.
    fn relation(&self, id: &NominalId) -> Result<Vec<RelationEntry>>;

    fn lineages_of(&self, id: &Digest) -> Result<Vec<Digest>>;
    /// A lineage's entries, ordered by seq.
    fn lineage(&self, lineage: &Digest) -> Result<Vec<LineageEntry>>;

    fn annotations_of(&self, subject: &Digest) -> Result<Vec<Annotation>>;
    fn facts_of(&self, subject: &Digest) -> Result<Vec<Fact>>;

    fn meta(&self, key: &str) -> Result<Option<Vec<u8>>>;
    /// Meta rows whose key starts with `prefix`, sorted by key.
    fn meta_prefix(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>>;
}

/// Writes, inside a transaction. Every write is part of one atomic commit.
pub trait WriteTx: ReadTx {
    /// Idempotent: putting an existing record changes nothing.
    fn put_record(&mut self, record: &Record) -> Result<()>;
    fn put_name(&mut self, entry: &NameEntry) -> Result<()>;
    fn remove_name(&mut self, path: &str) -> Result<()>;
    fn append_log(&mut self, entry: &LogEntry) -> Result<()>;
    fn put_nominal(&mut self, nominal: &Nominal) -> Result<()>;
    fn put_relation(&mut self, entry: &RelationEntry) -> Result<()>;
    fn put_lineage(&mut self, entry: &LineageEntry) -> Result<()>;
    fn put_annotation(&mut self, a: &Annotation) -> Result<()>;
    fn put_fact(&mut self, f: &Fact) -> Result<()>;
    fn set_meta(&mut self, key: &str, value: &[u8]) -> Result<()>;
    fn remove_meta(&mut self, key: &str) -> Result<()>;
}

/// A storage backend. `read` and `write` run a closure inside a transaction;
/// `write` commits when the closure returns `Ok` and rolls back otherwise.
pub trait Backend {
    /// A short name for reports: `sqlite`, `redb`, …
    fn name(&self) -> &'static str;
    fn read<R>(&self, f: impl FnOnce(&dyn ReadTx) -> Result<R>) -> Result<R>;
    fn write<R>(&self, f: impl FnOnce(&mut dyn WriteTx) -> Result<R>) -> Result<R>;
    /// Bytes on disk, if known.
    fn size_on_disk(&self) -> Option<u64> {
        None
    }
}
