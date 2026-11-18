//! The redb backend: a pure-Rust, copy-on-write B-tree key-value store
//! (SPEC-001 §12). The tables and the store logic are in [`crate::kv`]; this
//! file only adapts redb's transactions. See `redb_store.md`.

use std::path::{Path, PathBuf};

use ::redb::{
    Database, ReadOnlyDatabase, ReadTransaction, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition, WriteTransaction,
};

use crate::backend::{Backend, ReadTx, Result, WriteTx};
use crate::error::StoreError;
use crate::kv::{Kv, KvWrite, Table};

type Tbl = TableDefinition<'static, &'static [u8], &'static [u8]>;

fn def(t: Table) -> Tbl {
    TableDefinition::new(t.name())
}

fn be<E: std::fmt::Display>(e: E) -> StoreError {
    StoreError::Backend(e.to_string())
}

/// The redb backend.
pub struct Redb {
    db: Db,
    path: PathBuf,
}

/// A redb handle: read-write, or read-only (another process may read a file
/// that no process holds for writing, B-017).
enum Db {
    Rw(Database),
    Ro(ReadOnlyDatabase),
}

impl Redb {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = Database::create(path.as_ref()).map_err(be)?;
        let w = db.begin_write().map_err(be)?;
        for t in Table::ALL {
            w.open_table(def(t)).map_err(be)?;
        }
        w.commit().map_err(be)?;
        Ok(Redb { db: Db::Rw(db), path: path.as_ref().to_path_buf() })
    }

    /// Opens an existing store **read-only**: nothing is created and every
    /// write fails. redb locks its file, so this fails while another process
    /// holds the store open for writing (B-017).
    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Self> {
        let db = ReadOnlyDatabase::open(path.as_ref()).map_err(be)?;
        Ok(Redb { db: Db::Ro(db), path: path.as_ref().to_path_buf() })
    }

    /// Compacts the file (redb does not reclaim space on its own). Benchmarks
    /// report size both before and after.
    pub fn compact(&mut self) -> Result<bool> {
        match &mut self.db {
            Db::Rw(db) => db.compact().map_err(be),
            Db::Ro(_) => Err(StoreError::Backend("the store is open read-only".into())),
        }
    }
}

fn scan_table<T: ReadableTable<&'static [u8], &'static [u8]>>(
    table: &T,
    from: &[u8],
    prefix: &[u8],
    limit: usize,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut out = Vec::new();
    for item in table.range::<&[u8]>(from..).map_err(be)? {
        let (k, v) = item.map_err(be)?;
        if !k.value().starts_with(prefix) || out.len() == limit {
            break;
        }
        out.push((k.value().to_vec(), v.value().to_vec()));
    }
    Ok(out)
}

struct RedbRead {
    txn: ReadTransaction,
}

impl Kv for RedbRead {
    fn get(&self, t: Table, k: &[u8]) -> Result<Option<Vec<u8>>> {
        let table = self.txn.open_table(def(t)).map_err(be)?;
        Ok(table.get(k).map_err(be)?.map(|v| v.value().to_vec()))
    }

    fn scan(&self, t: Table, from: &[u8], prefix: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        scan_table(&self.txn.open_table(def(t)).map_err(be)?, from, prefix, limit)
    }

    fn last_key(&self, t: Table) -> Result<Option<Vec<u8>>> {
        let table = self.txn.open_table(def(t)).map_err(be)?;
        Ok(table.last().map_err(be)?.map(|(k, _)| k.value().to_vec()))
    }

    fn len(&self, t: Table) -> Result<u64> {
        self.txn.open_table(def(t)).map_err(be)?.len().map_err(be)
    }
}

struct RedbWrite<'a> {
    txn: &'a WriteTransaction,
}

impl Kv for RedbWrite<'_> {
    fn get(&self, t: Table, k: &[u8]) -> Result<Option<Vec<u8>>> {
        let table = self.txn.open_table(def(t)).map_err(be)?;
        Ok(table.get(k).map_err(be)?.map(|v| v.value().to_vec()))
    }

    fn scan(&self, t: Table, from: &[u8], prefix: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        scan_table(&self.txn.open_table(def(t)).map_err(be)?, from, prefix, limit)
    }

    fn last_key(&self, t: Table) -> Result<Option<Vec<u8>>> {
        let table = self.txn.open_table(def(t)).map_err(be)?;
        Ok(table.last().map_err(be)?.map(|(k, _)| k.value().to_vec()))
    }

    fn len(&self, t: Table) -> Result<u64> {
        self.txn.open_table(def(t)).map_err(be)?.len().map_err(be)
    }
}

impl KvWrite for RedbWrite<'_> {
    fn put(&mut self, t: Table, k: &[u8], v: &[u8]) -> Result<()> {
        self.txn.open_table(def(t)).map_err(be)?.insert(k, v).map_err(be)?;
        Ok(())
    }

    fn del(&mut self, t: Table, k: &[u8]) -> Result<()> {
        self.txn.open_table(def(t)).map_err(be)?.remove(k).map_err(be)?;
        Ok(())
    }
}

impl Backend for Redb {
    fn name(&self) -> &'static str {
        "redb"
    }

    fn read<R>(&self, f: impl FnOnce(&dyn ReadTx) -> Result<R>) -> Result<R> {
        let txn = match &self.db {
            Db::Rw(db) => db.begin_read(),
            Db::Ro(db) => db.begin_read(),
        };
        let r = RedbRead { txn: txn.map_err(be)? };
        f(&r)
    }

    fn write<R>(&self, f: impl FnOnce(&mut dyn WriteTx) -> Result<R>) -> Result<R> {
        let Db::Rw(db) = &self.db else {
            return Err(StoreError::Backend("the store is open read-only".into()));
        };
        let txn = db.begin_write().map_err(be)?;
        let out = f(&mut RedbWrite { txn: &txn });
        match out {
            Ok(v) => {
                txn.commit().map_err(be)?;
                Ok(v)
            }
            Err(e) => {
                txn.abort().map_err(be)?;
                Err(e)
            }
        }
    }

    fn size_on_disk(&self) -> Option<u64> {
        std::fs::metadata(&self.path).ok().map(|m| m.len())
    }
}
