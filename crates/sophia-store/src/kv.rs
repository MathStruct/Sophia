//! The store over any ordered byte-to-byte key-value engine (redb, LMDB):
//! the tables, their key layouts, and the `ReadTx`/`WriteTx` logic, written
//! once. An engine implements [`Kv`] and [`KvWrite`]. See `kv.md`.

use sophia_hash::{Digest, NominalId, Record};

use crate::backend::{ReadTx, Result, WriteTx};
use crate::codec::*;
use crate::error::StoreError;
use crate::types::*;

/// The tables. Values use the store codec; index tables have empty values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Table {
    Rec,
    /// component ‖ index(be) ‖ id
    Comp,
    /// kind ‖ 0 ‖ id
    Kind,
    /// dst ‖ src
    RefRev,
    /// path
    Name,
    /// target ‖ path
    NameTgt,
    /// seq(be)
    Log,
    Nom,
    /// nominal ‖ member
    Rel,
    /// lineage ‖ id
    Lin,
    /// id ‖ lineage
    LinId,
    Ann,
    /// subject ‖ id
    AnnSub,
    Fact,
    /// subject ‖ id
    FactSub,
    Meta,
}

impl Table {
    pub const ALL: [Table; 16] = [
        Table::Rec, Table::Comp, Table::Kind, Table::RefRev, Table::Name, Table::NameTgt, Table::Log, Table::Nom,
        Table::Rel, Table::Lin, Table::LinId, Table::Ann, Table::AnnSub, Table::Fact, Table::FactSub, Table::Meta,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Table::Rec => "record",
            Table::Comp => "component",
            Table::Kind => "kind",
            Table::RefRev => "ref_rev",
            Table::Name => "name",
            Table::NameTgt => "name_target",
            Table::Log => "log",
            Table::Nom => "nominal",
            Table::Rel => "relation",
            Table::Lin => "lineage",
            Table::LinId => "lineage_by_id",
            Table::Ann => "annotation",
            Table::AnnSub => "annotation_subject",
            Table::Fact => "fact",
            Table::FactSub => "fact_subject",
            Table::Meta => "meta",
        }
    }
}

/// Reads an engine must provide.
pub trait Kv {
    fn get(&self, t: Table, k: &[u8]) -> Result<Option<Vec<u8>>>;
    /// Entries with key ≥ `from` that start with `prefix`, in key order, up to `limit`.
    fn scan(&self, t: Table, from: &[u8], prefix: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
    fn last_key(&self, t: Table) -> Result<Option<Vec<u8>>>;
    fn len(&self, t: Table) -> Result<u64>;
}

/// Writes an engine must provide.
pub trait KvWrite: Kv {
    fn put(&mut self, t: Table, k: &[u8], v: &[u8]) -> Result<()>;
    fn del(&mut self, t: Table, k: &[u8]) -> Result<()>;
}

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

fn tail_digest(key: &[u8]) -> Result<Digest> {
    key.len().checked_sub(32).and_then(|i| Digest::from_slice(&key[i..])).ok_or_else(|| StoreError::Codec("key".into()))
}

const ALL_ROWS: usize = usize::MAX;

impl<X: Kv> ReadTx for X {
    fn record(&self, id: &Digest) -> Result<Option<Record>> {
        self.get(Table::Rec, id.as_bytes())?.map(|b| decode_record(&b)).transpose()
    }

    fn has_record(&self, id: &Digest) -> Result<bool> {
        Ok(self.get(Table::Rec, id.as_bytes())?.is_some())
    }

    fn component(&self, component: &Digest) -> Result<Vec<Record>> {
        let mut out = Vec::new();
        for (k, _) in self.scan(Table::Comp, component.as_bytes(), component.as_bytes(), ALL_ROWS)? {
            if let Some(r) = self.record(&tail_digest(&k)?)? {
                out.push(r);
            }
        }
        Ok(out)
    }

    fn dependants(&self, id: &Digest) -> Result<Vec<Digest>> {
        self.scan(Table::RefRev, id.as_bytes(), id.as_bytes(), ALL_ROWS)?.iter().map(|(k, _)| tail_digest(k)).collect()
    }

    fn record_count(&self) -> Result<u64> {
        self.len(Table::Rec)
    }

    fn records_of_kind(&self, kind: Option<&str>, limit: usize) -> Result<Vec<Digest>> {
        match kind {
            Some(k) => {
                let p = cat(&[k.as_bytes(), &[0]]);
                self.scan(Table::Kind, &p, &p, limit)?.iter().map(|(k, _)| tail_digest(k)).collect()
            }
            None => self.scan(Table::Rec, &[], &[], limit)?.iter().map(|(k, _)| tail_digest(k)).collect(),
        }
    }

    fn name(&self, path: &str) -> Result<Option<NameEntry>> {
        self.get(Table::Name, path.as_bytes())?.map(|b| decode_name(&b)).transpose()
    }

    fn names_under(&self, prefix: &str, limit: usize) -> Result<Vec<NameEntry>> {
        let mut out = Vec::new();
        if prefix.is_empty() {
            for (_, v) in self.scan(Table::Name, &[], &[], limit)? {
                out.push(decode_name(&v)?);
            }
            return Ok(out);
        }
        if let Some(n) = self.name(prefix)? {
            out.push(n);
        }
        let p = format!("{prefix}.");
        for (_, v) in self.scan(Table::Name, p.as_bytes(), p.as_bytes(), limit.saturating_sub(out.len()))? {
            out.push(decode_name(&v)?);
        }
        Ok(out)
    }

    fn names_of(&self, target: &Digest) -> Result<Vec<NameEntry>> {
        let mut out = Vec::new();
        for (k, _) in self.scan(Table::NameTgt, target.as_bytes(), target.as_bytes(), ALL_ROWS)? {
            let path = std::str::from_utf8(&k[32..]).map_err(|_| StoreError::Codec("path".into()))?;
            if let Some(n) = self.name(path)? {
                out.push(n);
            }
        }
        Ok(out)
    }

    fn last_seq(&self) -> Result<u64> {
        Ok(self
            .last_key(Table::Log)?
            .map(|k| u64::from_be_bytes(k.as_slice().try_into().unwrap_or([0; 8])))
            .unwrap_or(0))
    }

    fn log_since(&self, seq: u64, limit: usize) -> Result<Vec<LogEntry>> {
        let from = (seq + 1).to_be_bytes();
        self.scan(Table::Log, &from, &[], limit)?.iter().map(|(_, v)| decode_log(v)).collect()
    }

    fn nominal(&self, id: &NominalId) -> Result<Option<Nominal>> {
        self.get(Table::Nom, id.as_bytes())?.map(|b| decode_nominal(&b)).transpose()
    }

    fn relation(&self, id: &NominalId) -> Result<Vec<RelationEntry>> {
        let mut out: Vec<RelationEntry> = self
            .scan(Table::Rel, id.as_bytes(), id.as_bytes(), ALL_ROWS)?
            .iter()
            .map(|(_, v)| decode_relation(v))
            .collect::<Result<_>>()?;
        out.retain(|e| e.removed_seq.is_none());
        out.sort_by_key(|e| (e.added_seq, e.member));
        Ok(out)
    }

    fn lineages_of(&self, id: &Digest) -> Result<Vec<Digest>> {
        self.scan(Table::LinId, id.as_bytes(), id.as_bytes(), ALL_ROWS)?.iter().map(|(k, _)| tail_digest(k)).collect()
    }

    fn lineage(&self, lineage: &Digest) -> Result<Vec<LineageEntry>> {
        let mut out: Vec<LineageEntry> = self
            .scan(Table::Lin, lineage.as_bytes(), lineage.as_bytes(), ALL_ROWS)?
            .iter()
            .map(|(_, v)| decode_lineage(v))
            .collect::<Result<_>>()?;
        out.sort_by_key(|e| (e.seq, e.id));
        Ok(out)
    }

    fn annotations_of(&self, subject: &Digest) -> Result<Vec<Annotation>> {
        let mut out = Vec::new();
        for (k, _) in self.scan(Table::AnnSub, subject.as_bytes(), subject.as_bytes(), ALL_ROWS)? {
            if let Some(b) = self.get(Table::Ann, tail_digest(&k)?.as_bytes())? {
                out.push(decode_annotation(&b)?);
            }
        }
        Ok(out)
    }

    fn facts_of(&self, subject: &Digest) -> Result<Vec<Fact>> {
        let mut out = Vec::new();
        for (k, _) in self.scan(Table::FactSub, subject.as_bytes(), subject.as_bytes(), ALL_ROWS)? {
            if let Some(b) = self.get(Table::Fact, tail_digest(&k)?.as_bytes())? {
                out.push(decode_fact(&b)?);
            }
        }
        out.sort_by_key(|f| (f.derived_at, f.id));
        Ok(out)
    }

    fn meta(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.get(Table::Meta, key.as_bytes())
    }

    fn meta_prefix(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>> {
        self.scan(Table::Meta, prefix.as_bytes(), prefix.as_bytes(), ALL_ROWS)?
            .into_iter()
            .map(|(k, v)| Ok((String::from_utf8(k).map_err(|_| StoreError::Codec("meta key".into()))?, v)))
            .collect()
    }
}

impl<X: KvWrite> WriteTx for X {
    fn put_record(&mut self, r: &Record) -> Result<()> {
        if self.has_record(&r.id)? {
            return Ok(());
        }
        self.put(Table::Rec, r.id.as_bytes(), &encode_record(r))?;
        self.put(Table::Comp, &cat(&[r.component.as_bytes(), &r.index.to_be_bytes(), r.id.as_bytes()]), &[])?;
        self.put(Table::Kind, &cat(&[r.kind.as_bytes(), &[0], r.id.as_bytes()]), &[])?;
        for d in &r.refs {
            self.put(Table::RefRev, &cat(&[d.as_bytes(), r.id.as_bytes()]), &[])?;
        }
        Ok(())
    }

    fn put_name(&mut self, n: &NameEntry) -> Result<()> {
        if let Some(old) = self.name(&n.path)? {
            self.del(Table::NameTgt, &cat(&[old.target.digest().as_bytes(), old.path.as_bytes()]))?;
        }
        self.put(Table::Name, n.path.as_bytes(), &encode_name(n))?;
        self.put(Table::NameTgt, &cat(&[n.target.digest().as_bytes(), n.path.as_bytes()]), &[])
    }

    fn remove_name(&mut self, path: &str) -> Result<()> {
        if let Some(old) = self.name(path)? {
            self.del(Table::NameTgt, &cat(&[old.target.digest().as_bytes(), path.as_bytes()]))?;
        }
        self.del(Table::Name, path.as_bytes())
    }

    fn append_log(&mut self, e: &LogEntry) -> Result<()> {
        self.put(Table::Log, &e.seq.to_be_bytes(), &encode_log(e))
    }

    fn put_nominal(&mut self, n: &Nominal) -> Result<()> {
        self.put(Table::Nom, n.id.as_bytes(), &encode_nominal(n))
    }

    fn put_relation(&mut self, e: &RelationEntry) -> Result<()> {
        self.put(Table::Rel, &cat(&[e.nominal.as_bytes(), e.member.as_bytes()]), &encode_relation(e))
    }

    fn put_lineage(&mut self, e: &LineageEntry) -> Result<()> {
        let key = cat(&[e.lineage.as_bytes(), e.id.as_bytes()]);
        if self.get(Table::Lin, &key)?.is_some() {
            return Ok(());
        }
        self.put(Table::Lin, &key, &encode_lineage(e))?;
        self.put(Table::LinId, &cat(&[e.id.as_bytes(), e.lineage.as_bytes()]), &[])
    }

    fn put_annotation(&mut self, a: &Annotation) -> Result<()> {
        self.put(Table::Ann, a.id.as_bytes(), &encode_annotation(a))?;
        for s in &a.subjects {
            self.put(Table::AnnSub, &cat(&[s.as_bytes(), a.id.as_bytes()]), &[])?;
        }
        Ok(())
    }

    fn put_fact(&mut self, f: &Fact) -> Result<()> {
        if self.get(Table::Fact, f.id.as_bytes())?.is_some() {
            return Ok(());
        }
        self.put(Table::Fact, f.id.as_bytes(), &encode_fact(f))?;
        self.put(Table::FactSub, &cat(&[f.subject.as_bytes(), f.id.as_bytes()]), &[])
    }

    fn set_meta(&mut self, key: &str, value: &[u8]) -> Result<()> {
        self.put(Table::Meta, key.as_bytes(), value)
    }

    fn remove_meta(&mut self, key: &str) -> Result<()> {
        self.del(Table::Meta, key.as_bytes())
    }
}

