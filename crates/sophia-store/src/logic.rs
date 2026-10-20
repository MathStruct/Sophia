//! The store's logic, written once over any [`Backend`] (SPEC-001 §5 to §8).
//! See `logic.md`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sophia_hash::{Digest, NominalId, Record};

use crate::backend::{Backend, ReadTx, Result, WriteTx};
use crate::error::StoreError;
use crate::types::*;

/// Seconds since the Unix epoch.
pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// A fresh 32-byte nonce for a nominal entity. Uniqueness, not secrecy, is
/// what matters: time, process, a counter and an address are hashed together.
pub fn fresh_nonce() -> [u8; 32] {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let local = 0u8;
    let mut h = blake3::Hasher::new();
    h.update(b"sophia-nonce");
    h.update(&t.to_le_bytes());
    h.update(&std::process::id().to_le_bytes());
    h.update(&COUNTER.fetch_add(1, Ordering::Relaxed).to_le_bytes());
    h.update(&(&local as *const u8 as usize).to_le_bytes());
    h.update(format!("{:?}", std::thread::current().id()).as_bytes());
    *h.finalize().as_bytes()
}

/// Checks a dotted name path (SPEC-004 §3): non-empty segments that do not
/// start with a digit, made of letters, digits, `_` and `!`, or an operator
/// in backquotes (`` `+` ``).
pub fn validate_path(path: &str) -> Result<()> {
    let bad = |segment: &str| StoreError::InvalidPath { path: path.into(), segment: segment.into() };
    if path.is_empty() {
        return Err(bad(""));
    }
    for seg in split_path(path) {
        if seg.is_empty() {
            return Err(bad(seg));
        }
        if let Some(op) = seg.strip_prefix('`').and_then(|s| s.strip_suffix('`')) {
            if op.is_empty() || op.chars().any(|c| c.is_whitespace() || c.is_control() || c == '`') {
                return Err(bad(seg));
            }
            continue;
        }
        let mut chars = seg.chars();
        let first = chars.next().unwrap();
        if first.is_ascii_digit() || !(first.is_alphanumeric() || first == '_') {
            return Err(bad(seg));
        }
        if !chars.all(|c| c.is_alphanumeric() || c == '_' || c == '!') {
            return Err(bad(seg));
        }
    }
    Ok(())
}

/// Splits on dots outside backquotes.
pub fn split_path(path: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut start, mut quoted) = (0, false);
    for (i, c) in path.char_indices() {
        match c {
            '`' => quoted = !quoted,
            '.' if !quoted => {
                out.push(&path[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&path[start..]);
    out
}

/// The result of adding definitions with names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddReport {
    /// Records that were not already stored.
    pub new_records: usize,
    /// The seq of each added name, in input order.
    pub name_seqs: Vec<u64>,
}

/// The result of pointing a name at new content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateReport {
    pub old: Target,
    pub new: Digest,
    /// The name's version after the update (unchanged on a no-op).
    pub seq: u64,
    /// The new content was already what the name pointed at: nothing written.
    pub unchanged: bool,
    /// Definitions that still reference the old content (SP-002).
    pub dependants_of_old: Vec<Digest>,
}

/// The Sophia store over a backend.
pub struct Store<B> {
    backend: B,
    actor: String,
}

impl<B: Backend> Store<B> {
    pub fn new(backend: B) -> Self {
        Store { backend, actor: "sophia".into() }
    }

    /// Sets who is recorded as the author of changes in the log.
    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = actor.into();
        self
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn read<R>(&self, f: impl FnOnce(&dyn ReadTx) -> Result<R>) -> Result<R> {
        self.backend.read(f)
    }

    fn log(&self, tx: &mut dyn WriteTx, op: LogOp, subject: String, old: Option<Digest>, new: Option<Digest>) -> Result<u64> {
        let seq = tx.last_seq()? + 1;
        tx.append_log(&LogEntry { seq, op, subject, old, new, actor: self.actor.clone(), at: now() })?;
        Ok(seq)
    }

    fn put_new(&self, tx: &mut dyn WriteTx, records: &[Record]) -> Result<usize> {
        let mut new = 0;
        for r in records {
            if !tx.has_record(&r.id)? {
                tx.put_record(r)?;
                self.log(tx, LogOp::AddDefinition, r.kind.clone(), None, Some(r.id))?;
                new += 1;
            } else if let Some(stored) = tx.kind_of(&r.id)? {
                // SP-012: a kind is a function of content. The same content
                // under another kind means a frontend put a role in the kind.
                if stored != r.kind {
                    return Err(StoreError::KindConflict { id: r.id.to_hex(), stored, new: r.kind.clone() });
                }
            }
        }
        Ok(new)
    }

    /// Stores records without naming them. Returns how many were new.
    pub fn put_records(&self, records: &[Record]) -> Result<usize> {
        self.backend.write(|tx| self.put_new(tx, records))
    }

    /// Stores records and adds names to some of them: `(path, index into
    /// records, visibility)`. Each new name starts a lineage at its content.
    pub fn add(&self, records: &[Record], names: &[(String, usize, Visibility)]) -> Result<AddReport> {
        for (p, i, _) in names {
            validate_path(p)?;
            if *i >= records.len() {
                return Err(StoreError::NotFound(format!("record index {i}")));
            }
        }
        self.backend.write(|tx| {
            let new_records = self.put_new(tx, records)?;
            let mut name_seqs = Vec::new();
            for (path, i, vis) in names {
                if tx.name(path)?.is_some() {
                    return Err(StoreError::NameExists(path.clone()));
                }
                let id = records[*i].id;
                let seq = self.log(tx, LogOp::AddName, path.clone(), None, Some(id))?;
                tx.put_name(&NameEntry { path: path.clone(), target: Target::Content(id), visibility: *vis, seq })?;
                if tx.lineages_of(&id)?.is_empty() {
                    tx.put_lineage(&LineageEntry { lineage: id, id, seq, supersedes: None })?;
                }
                name_seqs.push(seq);
            }
            Ok(AddReport { new_records, name_seqs })
        })
    }

    /// Points an existing name at `records[target]` (an edit), if the name is
    /// still at version `expected`. A no-op if the content is unchanged.
    pub fn update(&self, path: &str, records: &[Record], target: usize, expected: u64) -> Result<UpdateReport> {
        let new = records.get(target).ok_or_else(|| StoreError::NotFound(format!("record index {target}")))?.id;
        self.backend.write(|tx| {
            let name = tx.name(path)?.ok_or_else(|| StoreError::NoSuchName(path.into()))?;
            if name.seq != expected {
                return Err(StoreError::Conflict { path: path.into(), expected, found: name.seq });
            }
            if name.target == Target::Content(new) {
                return Ok(UpdateReport {
                    old: name.target,
                    new,
                    seq: name.seq,
                    unchanged: true,
                    dependants_of_old: Vec::new(),
                });
            }
            self.put_new(tx, records)?;
            let old = name.target;
            let seq = self.log(tx, LogOp::MoveName, path.into(), Some(old.digest()), Some(new))?;
            tx.put_name(&NameEntry { target: Target::Content(new), seq, ..name })?;
            let lineages = tx.lineages_of(&old.digest())?;
            if lineages.is_empty() {
                tx.put_lineage(&LineageEntry { lineage: new, id: new, seq, supersedes: None })?;
            }
            for l in lineages {
                tx.put_lineage(&LineageEntry { lineage: l, id: new, seq, supersedes: Some(old.digest()) })?;
            }
            let dependants_of_old = tx.dependants(&old.digest())?;
            Ok(UpdateReport { old, new, seq, unchanged: false, dependants_of_old })
        })
    }

    /// Adds another name for an existing target.
    pub fn alias(&self, path: &str, target: Target, visibility: Visibility) -> Result<u64> {
        validate_path(path)?;
        self.backend.write(|tx| {
            if tx.name(path)?.is_some() {
                return Err(StoreError::NameExists(path.into()));
            }
            let exists = match target {
                Target::Content(d) => tx.has_record(&d)?,
                Target::Nominal(n) => tx.nominal(&n)?.is_some(),
            };
            if !exists {
                return Err(StoreError::NotFound(format!("{:?}", target)));
            }
            let seq = self.log(tx, LogOp::AddName, path.into(), None, Some(target.digest()))?;
            tx.put_name(&NameEntry { path: path.into(), target, visibility, seq })?;
            Ok(seq)
        })
    }

    /// Moves a name. No id changes. Returns the new name's seq.
    pub fn rename(&self, from: &str, to: &str, expected: Option<u64>) -> Result<u64> {
        validate_path(to)?;
        self.backend.write(|tx| {
            let name = tx.name(from)?.ok_or_else(|| StoreError::NoSuchName(from.into()))?;
            if let Some(e) = expected.filter(|e| *e != name.seq) {
                return Err(StoreError::Conflict { path: from.into(), expected: e, found: name.seq });
            }
            if tx.name(to)?.is_some() {
                return Err(StoreError::NameExists(to.into()));
            }
            let seq = self.log(tx, LogOp::RemoveName, from.into(), Some(name.target.digest()), None)?;
            tx.remove_name(from)?;
            let seq2 = self.log(tx, LogOp::AddName, to.into(), None, Some(name.target.digest()))?;
            debug_assert!(seq2 > seq);
            tx.put_name(&NameEntry { path: to.into(), seq: seq2, ..name })?;
            Ok(seq2)
        })
    }

    /// Removes a name. The definition stays.
    pub fn remove(&self, path: &str, expected: Option<u64>) -> Result<u64> {
        self.backend.write(|tx| {
            let name = tx.name(path)?.ok_or_else(|| StoreError::NoSuchName(path.into()))?;
            if let Some(e) = expected.filter(|e| *e != name.seq) {
                return Err(StoreError::Conflict { path: path.into(), expected: e, found: name.seq });
            }
            let seq = self.log(tx, LogOp::RemoveName, path.into(), Some(name.target.digest()), None)?;
            tx.remove_name(path)?;
            Ok(seq)
        })
    }

    pub fn resolve(&self, path: &str) -> Result<Option<NameEntry>> {
        self.backend.read(|tx| tx.name(path))
    }

    pub fn record(&self, id: &Digest) -> Result<Option<Record>> {
        self.backend.read(|tx| tx.record(id))
    }

    /// Re-verifies a stored definition by rehashing its whole component.
    pub fn verify(&self, id: &Digest) -> Result<bool> {
        self.backend.read(|tx| {
            let r = tx.record(id)?.ok_or_else(|| StoreError::NotFound(id.to_hex()))?;
            let members = tx.component(&r.component)?;
            let refs: Vec<&Record> = members.iter().collect();
            Ok(members.len() as u32 == r.component_size && Record::verify(&refs))
        })
    }

    /// Creates a nominal entity with a fresh id and names it.
    pub fn new_nominal(&self, lang: &str, kind: &str, path: &str) -> Result<NominalId> {
        validate_path(path)?;
        let id = sophia_hash::merkle::nominal_id(lang, kind, &fresh_nonce());
        self.backend.write(|tx| {
            if tx.name(path)?.is_some() {
                return Err(StoreError::NameExists(path.into()));
            }
            let seq = self.log(tx, LogOp::NewNominal, path.into(), None, Some(id))?;
            tx.put_nominal(&Nominal { id, lang: lang.into(), kind: kind.into(), label: path.into(), created_seq: seq, version: seq })?;
            tx.set_meta(&format!("nominal-index/{}", id.to_hex()), b"")?;
            let seq = self.log(tx, LogOp::AddName, path.into(), None, Some(id))?;
            tx.put_name(&NameEntry { path: path.into(), target: Target::Nominal(id), visibility: Visibility::Public, seq })?;
            Ok(id)
        })
    }

    /// Registers a nominal entity with a known id (a built-in). Idempotent.
    pub fn ensure_nominal(&self, id: NominalId, lang: &str, kind: &str, label: &str) -> Result<()> {
        self.backend.write(|tx| {
            if tx.nominal(&id)?.is_none() {
                let seq = self.log(tx, LogOp::NewNominal, label.into(), None, Some(id))?;
                tx.put_nominal(&Nominal { id, lang: lang.into(), kind: kind.into(), label: label.into(), created_seq: seq, version: seq })?;
                tx.set_meta(&format!("nominal-index/{}", id.to_hex()), b"")?;
            }
            Ok(())
        })
    }

    pub fn nominal(&self, id: &NominalId) -> Result<Option<Nominal>> {
        self.backend.read(|tx| tx.nominal(id))
    }

    /// Adds a method (a stored definition) to a generic function's table.
    /// Changes no definition id; bumps the table's version.
    pub fn add_method(&self, generic: &NominalId, method: &Digest) -> Result<u64> {
        self.backend.write(|tx| {
            let mut n = tx.nominal(generic)?.ok_or_else(|| StoreError::NotFound(generic.to_hex()))?;
            if !tx.has_record(method)? {
                return Err(StoreError::NotFound(method.to_hex()));
            }
            if tx.relation(generic)?.iter().any(|e| e.member == *method) {
                return Ok(n.version);
            }
            let seq = self.log(tx, LogOp::AddMethod, generic.to_hex(), None, Some(*method))?;
            tx.put_relation(&RelationEntry { nominal: *generic, member: *method, added_seq: seq, removed_seq: None })?;
            n.version = seq;
            tx.put_nominal(&n)?;
            Ok(seq)
        })
    }

    /// Adds many members to a nominal entity's relation in one transaction
    /// (a library's axioms, its proofs). Members already present are
    /// skipped. Returns the relation's version.
    pub fn add_members(&self, nominal: &NominalId, members: &[Digest]) -> Result<u64> {
        self.backend.write(|tx| {
            let mut n = tx.nominal(nominal)?.ok_or_else(|| StoreError::NotFound(nominal.to_hex()))?;
            let mut present: std::collections::HashSet<Digest> = tx.relation(nominal)?.into_iter().map(|e| e.member).collect();
            for m in members {
                if !present.insert(*m) {
                    continue;
                }
                if !tx.has_record(m)? {
                    return Err(StoreError::NotFound(m.to_hex()));
                }
                let seq = self.log(tx, LogOp::AddMethod, nominal.to_hex(), None, Some(*m))?;
                tx.put_relation(&RelationEntry { nominal: *nominal, member: *m, added_seq: seq, removed_seq: None })?;
                n.version = seq;
            }
            tx.put_nominal(&n)?;
            Ok(n.version)
        })
    }

    pub fn remove_method(&self, generic: &NominalId, method: &Digest) -> Result<u64> {
        self.backend.write(|tx| {
            let mut n = tx.nominal(generic)?.ok_or_else(|| StoreError::NotFound(generic.to_hex()))?;
            let Some(mut e) = tx.relation(generic)?.into_iter().find(|e| e.member == *method) else {
                return Err(StoreError::NotFound(method.to_hex()));
            };
            let seq = self.log(tx, LogOp::RemoveMethod, generic.to_hex(), Some(*method), None)?;
            e.removed_seq = Some(seq);
            tx.put_relation(&e)?;
            n.version = seq;
            tx.put_nominal(&n)?;
            Ok(seq)
        })
    }

    /// A generic function's current methods.
    pub fn methods(&self, generic: &NominalId) -> Result<Vec<Digest>> {
        self.backend.read(|tx| Ok(tx.relation(generic)?.into_iter().map(|e| e.member).collect()))
    }

    pub fn annotate(&self, a: &Annotation) -> Result<u64> {
        self.backend.write(|tx| {
            let seq = self.log(tx, LogOp::Annotate, a.kind.clone(), None, Some(a.id))?;
            tx.put_annotation(a)?;
            Ok(seq)
        })
    }

    /// Adds many annotations in one transaction, logged one by one. Returns
    /// the last seq.
    pub fn annotate_many(&self, annotations: &[Annotation]) -> Result<u64> {
        self.backend.write(|tx| {
            let mut seq = tx.last_seq()?;
            for a in annotations {
                seq = self.log(tx, LogOp::Annotate, a.kind.clone(), None, Some(a.id))?;
                tx.put_annotation(a)?;
            }
            Ok(seq)
        })
    }

    pub fn record_fact(&self, f: &Fact) -> Result<u64> {
        self.backend.write(|tx| {
            let seq = self.log(tx, LogOp::Fact, f.kind.clone(), None, Some(f.id))?;
            tx.put_fact(f)?;
            Ok(seq)
        })
    }

    /// A fact is stale when a relation it depended on has a newer version.
    /// Facts about immutable content are otherwise never wrong (SPEC-001 §8).
    pub fn is_stale(&self, f: &Fact) -> Result<bool> {
        self.backend.read(|tx| {
            for i in &f.inputs {
                if let Input::Relation { nominal, version } = i {
                    match tx.nominal(nominal)? {
                        Some(n) if n.version == *version => {}
                        _ => return Ok(true),
                    }
                }
            }
            Ok(false)
        })
    }

    /// The current version of a nominal entity's relation, for recording as
    /// a fact's input.
    pub fn relation_version(&self, id: &NominalId) -> Result<u64> {
        self.backend.read(|tx| tx.nominal(id)?.map(|n| n.version).ok_or_else(|| StoreError::NotFound(id.to_hex())))
    }

    /// Log entries after `seq`, at most `limit` (SPEC-004 §6).
    pub fn changes_since(&self, seq: u64, limit: usize) -> Result<Changes> {
        self.backend.read(|tx| {
            let mut entries = tx.log_since(seq, limit + 1)?;
            let more = entries.len() > limit;
            entries.truncate(limit);
            let next = entries.last().map(|e| e.seq).unwrap_or(seq);
            Ok(Changes { entries, next, more })
        })
    }

    pub fn last_seq(&self) -> Result<u64> {
        self.backend.read(|tx| tx.last_seq())
    }

    pub fn set_meta(&self, key: &str, value: &[u8]) -> Result<()> {
        self.backend.write(|tx| tx.set_meta(key, value))
    }

    pub fn meta(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.backend.read(|tx| tx.meta(key))
    }

    pub fn meta_prefix(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>> {
        self.backend.read(|tx| tx.meta_prefix(prefix))
    }

    /// Takes an immutable, content-addressed snapshot of the world: every
    /// nominal entity's current member set (SPEC-006 §2). The id depends only
    /// on the member sets, so two stores with the same tables get the same
    /// world. Builds, test results and proofs cite a world id, which keeps
    /// them reproducible when method tables change later.
    pub fn snapshot_world(&self) -> Result<Digest> {
        self.backend.write(|tx| {
            let mut entries: Vec<(Digest, Vec<Digest>)> = Vec::new();
            for (k, _) in tx.meta_prefix("nominal-index/")? {
                if let Ok(id) = k["nominal-index/".len()..].parse::<Digest>() {
                    let mut members: Vec<Digest> = tx.relation(&id)?.into_iter().map(|e| e.member).collect();
                    members.sort();
                    entries.push((id, members));
                }
            }
            entries.sort();
            let mut w = sophia_hash::canonical::Writer::new();
            w.str("sophia", "world-v0");
            for (id, members) in &entries {
                let o = w.open("nominal");
                w.field("id", id.as_bytes());
                for m in members {
                    w.field("m", m.as_bytes());
                }
                w.close(o);
            }
            let bytes = w.finish();
            let id = Digest::of(&bytes);
            if tx.meta(&format!("world/{}", id.to_hex()))?.is_none() {
                tx.set_meta(&format!("world/{}", id.to_hex()), &bytes)?;
            }
            Ok(id)
        })
    }

    /// The members a nominal entity had in a world snapshot.
    pub fn world_members(&self, world: &Digest, nominal: &NominalId) -> Result<Vec<Digest>> {
        let bytes = self.meta(&format!("world/{}", world.to_hex()))?.ok_or_else(|| StoreError::NotFound(format!("world {}", world.to_hex())))?;
        let mut r = sophia_hash::canonical::Reader::new(&bytes);
        r.expect_str("sophia").map_err(|e| StoreError::Codec(e.to_string()))?;
        while !r.is_empty() {
            let mut n = r.enter("nominal").map_err(|e| StoreError::Codec(e.to_string()))?;
            let id = Digest::from_slice(n.expect("id").map_err(|e| StoreError::Codec(e.to_string()))?).ok_or_else(|| StoreError::Codec("world".into()))?;
            if id == *nominal {
                let mut out = Vec::new();
                while !n.is_empty() {
                    out.push(Digest::from_slice(n.expect("m").map_err(|e| StoreError::Codec(e.to_string()))?).ok_or_else(|| StoreError::Codec("world".into()))?);
                }
                return Ok(out);
            }
        }
        Ok(Vec::new())
    }

    /// Writes settings rows (key, Some(value)) or removes them (key, None)
    /// in one transaction, logged as one `setting` change, so readers see a
    /// profile change through `changes_since` (SPEC-005 §2). Returns its seq.
    pub fn set_settings(&self, subject: &str, rows: &[(String, Option<String>)]) -> Result<u64> {
        self.backend.write(|tx| {
            for (k, v) in rows {
                match v {
                    Some(v) => tx.set_meta(k, v.as_bytes())?,
                    None => tx.remove_meta(k)?,
                }
            }
            self.log(tx, LogOp::Setting, subject.into(), None, None)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        for ok in ["a", "sample.add", "a.b_c!", "ops.`+`", "λ.μ", "_x"] {
            validate_path(ok).unwrap();
        }
        for bad in ["", "a..b", "1a", "a.b c", "a/b", "ops.``", ".a", "a."] {
            assert!(validate_path(bad).is_err(), "{bad}");
        }
        assert_eq!(split_path("ops.`.`.x"), vec!["ops", "`.`", "x"]);
    }

    #[test]
    fn nonces_differ() {
        assert_ne!(fresh_nonce(), fresh_nonce());
    }
}
