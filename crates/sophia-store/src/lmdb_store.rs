//! The LMDB backend (via `heed`): a memory-mapped B+tree with one writer and
//! zero-copy reads (SPEC-001 §12). The tables and the store logic are in
//! [`crate::kv`]; this file only adapts LMDB's transactions. See
//! `lmdb_store.md`.

use std::ops::Bound;
use std::path::{Path, PathBuf};

use heed::types::Bytes;
use heed::{Database, Env, EnvOpenOptions, RoTxn, RwTxn, WithTls};

use crate::backend::{Backend, ReadTx, Result, WriteTx};
use crate::error::StoreError;
use crate::kv::{Kv, KvWrite, Table};

type Db = Database<Bytes, Bytes>;

fn be<E: std::fmt::Display>(e: E) -> StoreError {
    StoreError::Backend(e.to_string())
}

/// The LMDB backend. LMDB maps the whole file into memory and needs a
/// maximum size up front; the default reserves 64 GiB of address space
/// (no disk is used until data is written).
pub struct Lmdb {
    env: Env,
    dbs: Vec<Db>,
    path: PathBuf,
}

impl Lmdb {
    pub const DEFAULT_MAP_SIZE: usize = 64 << 30;

    /// Opens (or creates) an LMDB environment in the directory `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_map_size(path, Self::DEFAULT_MAP_SIZE)
    }

    pub fn open_with_map_size(path: impl AsRef<Path>, map_size: usize) -> Result<Self> {
        std::fs::create_dir_all(path.as_ref()).map_err(be)?;
        // SAFETY: heed requires that the same environment is not opened twice
        // in one process with different options, and that the file is not
        // modified by other means while mapped. The store owns its directory.
        let env = unsafe {
            EnvOpenOptions::new().map_size(map_size).max_dbs(Table::ALL.len() as u32 + 4).open(path.as_ref())
        }
        .map_err(be)?;
        let mut w = env.write_txn().map_err(be)?;
        let mut dbs = Vec::new();
        for t in Table::ALL {
            dbs.push(env.create_database::<Bytes, Bytes>(&mut w, Some(t.name())).map_err(be)?);
        }
        w.commit().map_err(be)?;
        Ok(Lmdb { env, dbs, path: path.as_ref().to_path_buf() })
    }

    fn db(&self, t: Table) -> Db {
        self.dbs[Table::ALL.iter().position(|x| *x == t).expect("table")]
    }
}

fn scan(db: Db, txn: &RoTxn, from: &[u8], prefix: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut out = Vec::new();
    // LMDB rejects zero-length keys, also as a range start.
    let start = if from.is_empty() { Bound::Unbounded } else { Bound::Included(from) };
    let range = (start, Bound::Unbounded);
    for item in db.range(txn, &range).map_err(be)? {
        let (k, v) = item.map_err(be)?;
        if !k.starts_with(prefix) || out.len() == limit {
            break;
        }
        out.push((k.to_vec(), v.to_vec()));
    }
    Ok(out)
}

struct LmdbRead<'a> {
    store: &'a Lmdb,
    txn: RoTxn<'a, WithTls>,
}

impl Kv for LmdbRead<'_> {
    fn get(&self, t: Table, k: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.store.db(t).get(&self.txn, k).map_err(be)?.map(<[u8]>::to_vec))
    }

    fn scan(&self, t: Table, from: &[u8], prefix: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        scan(self.store.db(t), &self.txn, from, prefix, limit)
    }

    fn last_key(&self, t: Table) -> Result<Option<Vec<u8>>> {
        Ok(self.store.db(t).last(&self.txn).map_err(be)?.map(|(k, _)| k.to_vec()))
    }

    fn len(&self, t: Table) -> Result<u64> {
        self.store.db(t).len(&self.txn).map_err(be)
    }
}

struct LmdbWrite<'a> {
    store: &'a Lmdb,
    txn: RwTxn<'a>,
}

impl Kv for LmdbWrite<'_> {
    fn get(&self, t: Table, k: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.store.db(t).get(&self.txn, k).map_err(be)?.map(<[u8]>::to_vec))
    }

    fn scan(&self, t: Table, from: &[u8], prefix: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        scan(self.store.db(t), &self.txn, from, prefix, limit)
    }

    fn last_key(&self, t: Table) -> Result<Option<Vec<u8>>> {
        Ok(self.store.db(t).last(&self.txn).map_err(be)?.map(|(k, _)| k.to_vec()))
    }

    fn len(&self, t: Table) -> Result<u64> {
        self.store.db(t).len(&self.txn).map_err(be)
    }
}

impl KvWrite for LmdbWrite<'_> {
    fn put(&mut self, t: Table, k: &[u8], v: &[u8]) -> Result<()> {
        self.store.db(t).put(&mut self.txn, k, v).map_err(be)
    }

    fn del(&mut self, t: Table, k: &[u8]) -> Result<()> {
        self.store.db(t).delete(&mut self.txn, k).map_err(be).map(|_| ())
    }
}

impl Backend for Lmdb {
    fn name(&self) -> &'static str {
        "lmdb"
    }

    fn read<R>(&self, f: impl FnOnce(&dyn ReadTx) -> Result<R>) -> Result<R> {
        let r = LmdbRead { store: self, txn: self.env.read_txn().map_err(be)? };
        f(&r)
    }

    fn write<R>(&self, f: impl FnOnce(&mut dyn WriteTx) -> Result<R>) -> Result<R> {
        let mut w = LmdbWrite { store: self, txn: self.env.write_txn().map_err(be)? };
        let out = f(&mut w);
        match out {
            Ok(v) => {
                w.txn.commit().map_err(be)?;
                Ok(v)
            }
            Err(e) => {
                w.txn.abort();
                Err(e)
            }
        }
    }

    fn size_on_disk(&self) -> Option<u64> {
        std::fs::metadata(self.path.join("data.mdb")).ok().map(|m| m.len())
    }
}
