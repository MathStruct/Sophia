//! The redb backend: a pure-Rust, copy-on-write B-tree key-value store
//! (SPEC-001 §12). Every table maps bytes to bytes; values use the store
//! codec; secondary indexes are key-only tables. See `redb_store.md`.

use std::path::{Path, PathBuf};

use ::redb::{Database, ReadTransaction, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition, WriteTransaction};
use sophia_hash::{Digest, NominalId, Record};

use crate::backend::{Backend, ReadTx, Result, WriteTx};
use crate::codec::*;
use crate::error::StoreError;
use crate::types::*;

type Tbl = TableDefinition<'static, &'static [u8], &'static [u8]>;

const REC: Tbl = TableDefinition::new("record");
const COMP: Tbl = TableDefinition::new("component"); // component ‖ index(be) ‖ id
const KIND: Tbl = TableDefinition::new("kind"); // kind ‖ 0 ‖ id
const REFREV: Tbl = TableDefinition::new("ref_rev"); // dst ‖ src
const NAME: Tbl = TableDefinition::new("name"); // path
const NAMETGT: Tbl = TableDefinition::new("name_target"); // target ‖ path
const LOG: Tbl = TableDefinition::new("log"); // seq(be)
const NOM: Tbl = TableDefinition::new("nominal");
const REL: Tbl = TableDefinition::new("relation"); // nominal ‖ member
const LIN: Tbl = TableDefinition::new("lineage"); // lineage ‖ id
const LINID: Tbl = TableDefinition::new("lineage_by_id"); // id ‖ lineage
const ANN: Tbl = TableDefinition::new("annotation");
const ANNSUB: Tbl = TableDefinition::new("annotation_subject"); // subject ‖ id
const FACT: Tbl = TableDefinition::new("fact");
const FACTSUB: Tbl = TableDefinition::new("fact_subject"); // subject ‖ id
const META: Tbl = TableDefinition::new("meta");

const ALL: [Tbl; 16] = [REC, COMP, KIND, REFREV, NAME, NAMETGT, LOG, NOM, REL, LIN, LINID, ANN, ANNSUB, FACT, FACTSUB, META];

fn be<E: std::fmt::Display>(e: E) -> StoreError {
    StoreError::Backend(e.to_string())
}

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

fn tail_digest(key: &[u8]) -> Result<Digest> {
    key.len().checked_sub(32).and_then(|i| Digest::from_slice(&key[i..])).ok_or_else(|| StoreError::Codec("key".into()))
}

/// The redb backend.
pub struct Redb {
    db: Database,
    path: PathBuf,
}

impl Redb {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = Database::create(path.as_ref()).map_err(be)?;
        let w = db.begin_write().map_err(be)?;
        for t in ALL {
            w.open_table(t).map_err(be)?;
        }
        w.commit().map_err(be)?;
        Ok(Redb { db, path: path.as_ref().to_path_buf() })
    }
}

/// The two primitive operations every table needs.
trait Kv {
    fn get(&self, t: Tbl, k: &[u8]) -> Result<Option<Vec<u8>>>;
    /// Entries with key ≥ `from` that start with `prefix`, up to `limit`.
    fn scan(&self, t: Tbl, from: &[u8], prefix: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
    fn last_key(&self, t: Tbl) -> Result<Option<Vec<u8>>>;
    fn len(&self, t: Tbl) -> Result<u64>;
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
    fn get(&self, t: Tbl, k: &[u8]) -> Result<Option<Vec<u8>>> {
        let table = self.txn.open_table(t).map_err(be)?;
        Ok(table.get(k).map_err(be)?.map(|v| v.value().to_vec()))
    }

    fn scan(&self, t: Tbl, from: &[u8], prefix: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        scan_table(&self.txn.open_table(t).map_err(be)?, from, prefix, limit)
    }

    fn last_key(&self, t: Tbl) -> Result<Option<Vec<u8>>> {
        let table = self.txn.open_table(t).map_err(be)?;
        Ok(table.last().map_err(be)?.map(|(k, _)| k.value().to_vec()))
    }

    fn len(&self, t: Tbl) -> Result<u64> {
        self.txn.open_table(t).map_err(be)?.len().map_err(be)
    }
}

struct RedbWrite<'a> {
    txn: &'a WriteTransaction,
}

impl Kv for RedbWrite<'_> {
    fn get(&self, t: Tbl, k: &[u8]) -> Result<Option<Vec<u8>>> {
        let table = self.txn.open_table(t).map_err(be)?;
        Ok(table.get(k).map_err(be)?.map(|v| v.value().to_vec()))
    }

    fn scan(&self, t: Tbl, from: &[u8], prefix: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        scan_table(&self.txn.open_table(t).map_err(be)?, from, prefix, limit)
    }

    fn last_key(&self, t: Tbl) -> Result<Option<Vec<u8>>> {
        let table = self.txn.open_table(t).map_err(be)?;
        Ok(table.last().map_err(be)?.map(|(k, _)| k.value().to_vec()))
    }

    fn len(&self, t: Tbl) -> Result<u64> {
        self.txn.open_table(t).map_err(be)?.len().map_err(be)
    }
}

impl RedbWrite<'_> {
    fn put(&self, t: Tbl, k: &[u8], v: &[u8]) -> Result<()> {
        self.txn.open_table(t).map_err(be)?.insert(k, v).map_err(be)?;
        Ok(())
    }

    fn del(&self, t: Tbl, k: &[u8]) -> Result<()> {
        self.txn.open_table(t).map_err(be)?.remove(k).map_err(be)?;
        Ok(())
    }
}

const ALL_ROWS: usize = usize::MAX;

impl<T: Kv> ReadTx for T {
    fn record(&self, id: &Digest) -> Result<Option<Record>> {
        self.get(REC, id.as_bytes())?.map(|b| decode_record(&b)).transpose()
    }

    fn has_record(&self, id: &Digest) -> Result<bool> {
        Ok(self.get(REC, id.as_bytes())?.is_some())
    }

    fn component(&self, component: &Digest) -> Result<Vec<Record>> {
        let mut out = Vec::new();
        for (k, _) in self.scan(COMP, component.as_bytes(), component.as_bytes(), ALL_ROWS)? {
            if let Some(r) = self.record(&tail_digest(&k)?)? {
                out.push(r);
            }
        }
        Ok(out)
    }

    fn dependants(&self, id: &Digest) -> Result<Vec<Digest>> {
        self.scan(REFREV, id.as_bytes(), id.as_bytes(), ALL_ROWS)?.iter().map(|(k, _)| tail_digest(k)).collect()
    }

    fn record_count(&self) -> Result<u64> {
        self.len(REC)
    }

    fn records_of_kind(&self, kind: Option<&str>, limit: usize) -> Result<Vec<Digest>> {
        match kind {
            Some(k) => {
                let p = cat(&[k.as_bytes(), &[0]]);
                self.scan(KIND, &p, &p, limit)?.iter().map(|(k, _)| tail_digest(k)).collect()
            }
            None => self.scan(REC, &[], &[], limit)?.iter().map(|(k, _)| tail_digest(k)).collect(),
        }
    }

    fn name(&self, path: &str) -> Result<Option<NameEntry>> {
        self.get(NAME, path.as_bytes())?.map(|b| decode_name(&b)).transpose()
    }

    fn names_under(&self, prefix: &str, limit: usize) -> Result<Vec<NameEntry>> {
        let mut out = Vec::new();
        if prefix.is_empty() {
            for (_, v) in self.scan(NAME, &[], &[], limit)? {
                out.push(decode_name(&v)?);
            }
            return Ok(out);
        }
        if let Some(n) = self.name(prefix)? {
            out.push(n);
        }
        let p = format!("{prefix}.");
        for (_, v) in self.scan(NAME, p.as_bytes(), p.as_bytes(), limit.saturating_sub(out.len()))? {
            out.push(decode_name(&v)?);
        }
        Ok(out)
    }

    fn names_of(&self, target: &Digest) -> Result<Vec<NameEntry>> {
        let mut out = Vec::new();
        for (k, _) in self.scan(NAMETGT, target.as_bytes(), target.as_bytes(), ALL_ROWS)? {
            let path = std::str::from_utf8(&k[32..]).map_err(|_| StoreError::Codec("path".into()))?;
            if let Some(n) = self.name(path)? {
                out.push(n);
            }
        }
        Ok(out)
    }

    fn last_seq(&self) -> Result<u64> {
        Ok(self
            .last_key(LOG)?
            .map(|k| u64::from_be_bytes(k.as_slice().try_into().unwrap_or([0; 8])))
            .unwrap_or(0))
    }

    fn log_since(&self, seq: u64, limit: usize) -> Result<Vec<LogEntry>> {
        let from = (seq + 1).to_be_bytes();
        self.scan(LOG, &from, &[], limit)?.iter().map(|(_, v)| decode_log(v)).collect()
    }

    fn nominal(&self, id: &NominalId) -> Result<Option<Nominal>> {
        self.get(NOM, id.as_bytes())?.map(|b| decode_nominal(&b)).transpose()
    }

    fn relation(&self, id: &NominalId) -> Result<Vec<RelationEntry>> {
        let mut out: Vec<RelationEntry> = self
            .scan(REL, id.as_bytes(), id.as_bytes(), ALL_ROWS)?
            .iter()
            .map(|(_, v)| decode_relation(v))
            .collect::<Result<_>>()?;
        out.retain(|e| e.removed_seq.is_none());
        out.sort_by_key(|e| (e.added_seq, e.member));
        Ok(out)
    }

    fn lineages_of(&self, id: &Digest) -> Result<Vec<Digest>> {
        self.scan(LINID, id.as_bytes(), id.as_bytes(), ALL_ROWS)?.iter().map(|(k, _)| tail_digest(k)).collect()
    }

    fn lineage(&self, lineage: &Digest) -> Result<Vec<LineageEntry>> {
        let mut out: Vec<LineageEntry> = self
            .scan(LIN, lineage.as_bytes(), lineage.as_bytes(), ALL_ROWS)?
            .iter()
            .map(|(_, v)| decode_lineage(v))
            .collect::<Result<_>>()?;
        out.sort_by_key(|e| (e.seq, e.id));
        Ok(out)
    }

    fn annotations_of(&self, subject: &Digest) -> Result<Vec<Annotation>> {
        let mut out = Vec::new();
        for (k, _) in self.scan(ANNSUB, subject.as_bytes(), subject.as_bytes(), ALL_ROWS)? {
            if let Some(b) = self.get(ANN, tail_digest(&k)?.as_bytes())? {
                out.push(decode_annotation(&b)?);
            }
        }
        Ok(out)
    }

    fn facts_of(&self, subject: &Digest) -> Result<Vec<Fact>> {
        let mut out = Vec::new();
        for (k, _) in self.scan(FACTSUB, subject.as_bytes(), subject.as_bytes(), ALL_ROWS)? {
            if let Some(b) = self.get(FACT, tail_digest(&k)?.as_bytes())? {
                out.push(decode_fact(&b)?);
            }
        }
        out.sort_by_key(|f| (f.derived_at, f.id));
        Ok(out)
    }

    fn meta(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.get(META, key.as_bytes())
    }
}

impl WriteTx for RedbWrite<'_> {
    fn put_record(&mut self, r: &Record) -> Result<()> {
        if self.has_record(&r.id)? {
            return Ok(());
        }
        self.put(REC, r.id.as_bytes(), &encode_record(r))?;
        self.put(COMP, &cat(&[r.component.as_bytes(), &r.index.to_be_bytes(), r.id.as_bytes()]), &[])?;
        self.put(KIND, &cat(&[r.kind.as_bytes(), &[0], r.id.as_bytes()]), &[])?;
        for d in &r.refs {
            self.put(REFREV, &cat(&[d.as_bytes(), r.id.as_bytes()]), &[])?;
        }
        Ok(())
    }

    fn put_name(&mut self, n: &NameEntry) -> Result<()> {
        if let Some(old) = self.name(&n.path)? {
            self.del(NAMETGT, &cat(&[old.target.digest().as_bytes(), old.path.as_bytes()]))?;
        }
        self.put(NAME, n.path.as_bytes(), &encode_name(n))?;
        self.put(NAMETGT, &cat(&[n.target.digest().as_bytes(), n.path.as_bytes()]), &[])
    }

    fn remove_name(&mut self, path: &str) -> Result<()> {
        if let Some(old) = self.name(path)? {
            self.del(NAMETGT, &cat(&[old.target.digest().as_bytes(), path.as_bytes()]))?;
        }
        self.del(NAME, path.as_bytes())
    }

    fn append_log(&mut self, e: &LogEntry) -> Result<()> {
        self.put(LOG, &e.seq.to_be_bytes(), &encode_log(e))
    }

    fn put_nominal(&mut self, n: &Nominal) -> Result<()> {
        self.put(NOM, n.id.as_bytes(), &encode_nominal(n))
    }

    fn put_relation(&mut self, e: &RelationEntry) -> Result<()> {
        self.put(REL, &cat(&[e.nominal.as_bytes(), e.member.as_bytes()]), &encode_relation(e))
    }

    fn put_lineage(&mut self, e: &LineageEntry) -> Result<()> {
        let key = cat(&[e.lineage.as_bytes(), e.id.as_bytes()]);
        if self.get(LIN, &key)?.is_some() {
            return Ok(());
        }
        self.put(LIN, &key, &encode_lineage(e))?;
        self.put(LINID, &cat(&[e.id.as_bytes(), e.lineage.as_bytes()]), &[])
    }

    fn put_annotation(&mut self, a: &Annotation) -> Result<()> {
        self.put(ANN, a.id.as_bytes(), &encode_annotation(a))?;
        for s in &a.subjects {
            self.put(ANNSUB, &cat(&[s.as_bytes(), a.id.as_bytes()]), &[])?;
        }
        Ok(())
    }

    fn put_fact(&mut self, f: &Fact) -> Result<()> {
        if self.get(FACT, f.id.as_bytes())?.is_some() {
            return Ok(());
        }
        self.put(FACT, f.id.as_bytes(), &encode_fact(f))?;
        self.put(FACTSUB, &cat(&[f.subject.as_bytes(), f.id.as_bytes()]), &[])
    }

    fn set_meta(&mut self, key: &str, value: &[u8]) -> Result<()> {
        self.put(META, key.as_bytes(), value)
    }
}

impl Backend for Redb {
    fn name(&self) -> &'static str {
        "redb"
    }

    fn read<R>(&self, f: impl FnOnce(&dyn ReadTx) -> Result<R>) -> Result<R> {
        let r = RedbRead { txn: self.db.begin_read().map_err(be)? };
        f(&r)
    }

    fn write<R>(&self, f: impl FnOnce(&mut dyn WriteTx) -> Result<R>) -> Result<R> {
        let txn = self.db.begin_write().map_err(be)?;
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
