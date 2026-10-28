//! A folder store (SPEC-009): Sophia as the derived code-graph cache of a
//! folder of files. The files are the truth; a producer (Moonkale's index)
//! hands over finished records per file, and [`Store::replace_files`]
//! replaces everything a set of files contributed in one transaction. See
//! `folder.md`.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use sophia_hash::canonical::{Reader, Writer};
use sophia_hash::merkle::{component_digest, member_digest};
use sophia_hash::{Digest, NominalId, Record, Status};

use crate::backend::{Backend, ReadTx, WriteTx};
use crate::error::StoreError;
use crate::logic::{Store, validate_path};
use crate::types::*;

type Result<T> = std::result::Result<T, StoreError>;

/// Source files: the same records as `sophia-ingest`'s (SPEC-006 §1a).
pub const SOURCE_LANG: &str = "source";
pub const SOURCE_NORMALISER: &str = "sophia-source@0";
/// File manifests.
pub const MANIFEST_LANG: &str = "sophia-ingest";
/// `@1` (2026-10-24) adds the file's fact ids; `@0` manifests (no facts)
/// still decode.
pub const MANIFEST_NORMALISER: &str = "sophia-file-manifest@1";
pub const MANIFEST_KIND: &str = "file-ingest";

/// The store mode in which files are the truth (SPEC-009 §3).
pub const MODE_KEY: &str = "store/mode";
pub const MODE_DERIVED: &[u8] = b"derived";
const NORMALISER_KEY: &str = "folder/normaliser";
/// The log position where the truth moved into the store (SPEC-010 §3).
pub const TRUTH_SINCE_KEY: &str = "store/truth-since";

fn file_key(path: &str) -> String {
    format!("file/{path}")
}
fn owner_key(name: &str) -> String {
    format!("owner/{name}")
}
fn member_key(generic: &NominalId, member: &Digest) -> String {
    format!("member/{}/{}", generic.to_hex(), member.to_hex())
}
/// How many files contribute a fact.
fn fact_key(id: &Digest) -> String {
    format!("fact/{}", id.to_hex())
}
/// A fact that was stored before any file contributed it: never removed by
/// the folder writer.
fn fact_external_key(id: &Digest) -> String {
    format!("fact-external/{}", id.to_hex())
}
fn defined_prefix(id: &Digest) -> String {
    format!("defined-in/{}/", id.to_hex())
}

fn single(lang: &str, normaliser: &str, kind: &str, bytes: Vec<u8>, display: Vec<u8>, refs: Vec<Digest>) -> Record {
    let component = component_digest(lang, normaliser, &bytes);
    Record {
        id: member_digest(&component, 0),
        lang: lang.into(),
        normaliser: normaliser.into(),
        kind: kind.into(),
        component,
        index: 0,
        component_size: 1,
        bytes,
        display,
        status: Status::Complete,
        refs,
        nominal_refs: Vec::new(),
    }
}

/// A source file's record; its id is what a caller compares with the file
/// on disk to tell whether the store is stale for that file (SPEC-009 §7).
pub fn source_record(path: &str, text: &str) -> Record {
    single(SOURCE_LANG, SOURCE_NORMALISER, "source-file", text.as_bytes().to_vec(), path.as_bytes().to_vec(), Vec::new())
}

/// A generic function a file contributes methods to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileGeneric {
    /// The generic function's name (`jl.Pkg.f`).
    pub path: String,
    pub lang: String,
    /// Indices into the file's records.
    pub members: Vec<usize>,
}

/// Everything one file contributes, as the producer computed it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileIngest {
    /// Relative to the folder.
    pub path: String,
    /// The file's text; `None` when the file was deleted: everything it
    /// contributed is removed.
    pub text: Option<String>,
    /// The normaliser the records' ids come from (`moonkale-wp66@7`).
    pub normaliser: String,
    pub records: Vec<Record>,
    /// (record index, start byte, end byte) in the text.
    pub spans: Vec<(usize, u32, u32)>,
    /// Names of definitions: (name, record index).
    pub names: Vec<(String, usize)>,
    pub generics: Vec<FileGeneric>,
    /// Facts about this file's records (call edges to generic functions):
    /// stored as they are.
    pub facts: Vec<Fact>,
}

impl FileIngest {
    /// Each generic function's members, once each, as the manifest will
    /// store them.
    fn contributions(&self) -> BTreeMap<(String, String), BTreeSet<Digest>> {
        let mut c: BTreeMap<(String, String), BTreeSet<Digest>> = BTreeMap::new();
        for g in &self.generics {
            c.entry((g.path.clone(), g.lang.clone())).or_default().extend(g.members.iter().map(|i| self.records[*i].id));
        }
        c
    }
}

/// What a file contributed, as stored: the authority for "replace what came
/// from file F".
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileManifest {
    pub path: String,
    pub normaliser: String,
    pub source: Option<Digest>,
    pub names: Vec<(String, Digest)>,
    /// (generic name, language, member ids).
    pub generics: Vec<(String, String, Vec<Digest>)>,
    /// (record id, start, end) in the source.
    pub spans: Vec<(Digest, u32, u32)>,
    /// The ids of the facts the file contributed, sorted, each once.
    pub facts: Vec<Digest>,
}

impl FileManifest {
    /// Each generic function's members, once each (P-231).
    pub fn contributions(&self) -> BTreeMap<(String, String), BTreeSet<Digest>> {
        let mut c: BTreeMap<(String, String), BTreeSet<Digest>> = BTreeMap::new();
        for (g, lang, ms) in &self.generics {
            c.entry((g.clone(), lang.clone())).or_default().extend(ms.iter().copied());
        }
        c
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.str("path", &self.path);
        w.str("normaliser", &self.normaliser);
        if let Some(s) = &self.source {
            w.field("source", s.as_bytes());
        }
        for (n, id) in &self.names {
            let o = w.open("name");
            w.str("path", n);
            w.field("id", id.as_bytes());
            w.close(o);
        }
        for (n, lang, ms) in &self.generics {
            let o = w.open("generic");
            w.str("path", n);
            w.str("lang", lang);
            for m in ms {
                w.field("member", m.as_bytes());
            }
            w.close(o);
        }
        for (id, a, b) in &self.spans {
            let o = w.open("span");
            w.field("id", id.as_bytes());
            w.u32("start", *a);
            w.u32("end", *b);
            w.close(o);
        }
        for id in &self.facts {
            w.field("fact", id.as_bytes());
        }
        w.finish()
    }

    pub fn decode(bytes: &[u8]) -> Result<FileManifest> {
        let e = |x: sophia_hash::canonical::ReadError| StoreError::Codec(format!("file manifest: {x:?}"));
        let d = |b: &[u8]| Digest::from_slice(b).ok_or_else(|| StoreError::Codec("file manifest: digest".into()));
        let mut r = Reader::new(bytes);
        let mut m = FileManifest {
            path: r.expect_str("path").map_err(e)?.into(),
            normaliser: r.expect_str("normaliser").map_err(e)?.into(),
            ..FileManifest::default()
        };
        while !r.is_empty() {
            match r.peek_tag().map_err(e)? {
                "source" => m.source = Some(d(r.expect("source").map_err(e)?)?),
                "name" => {
                    let mut f = r.enter("name").map_err(e)?;
                    let n = f.expect_str("path").map_err(e)?.to_string();
                    m.names.push((n, d(f.expect("id").map_err(e)?)?));
                }
                "generic" => {
                    let mut f = r.enter("generic").map_err(e)?;
                    let n = f.expect_str("path").map_err(e)?.to_string();
                    let lang = f.expect_str("lang").map_err(e)?.to_string();
                    let mut ms = Vec::new();
                    while !f.is_empty() {
                        ms.push(d(f.expect("member").map_err(e)?)?);
                    }
                    m.generics.push((n, lang, ms));
                }
                "span" => {
                    let mut f = r.enter("span").map_err(e)?;
                    let id = d(f.expect("id").map_err(e)?)?;
                    let a = f.expect_u32("start").map_err(e)?;
                    let b = f.expect_u32("end").map_err(e)?;
                    m.spans.push((id, a, b));
                }
                "fact" => m.facts.push(d(r.expect("fact").map_err(e)?)?),
                t => return Err(StoreError::Codec(format!("file manifest: unexpected field {t}"))),
            }
        }
        Ok(m)
    }

    /// The manifest's record. It references its records and its source, so
    /// they stay reachable while the file does.
    pub fn record(&self) -> Record {
        let mut refs: Vec<Digest> = self.names.iter().map(|(_, id)| *id).collect();
        refs.extend(self.generics.iter().flat_map(|(_, _, ms)| ms.iter().copied()));
        refs.extend(self.spans.iter().map(|(id, _, _)| *id));
        refs.extend(self.source);
        refs.sort();
        refs.dedup();
        single(MANIFEST_LANG, MANIFEST_NORMALISER, MANIFEST_KIND, self.encode(), self.path.as_bytes().to_vec(), refs)
    }
}

/// What `replace_files` changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileReport {
    pub files: usize,
    pub facts_added: usize,
    pub facts_removed: usize,
    pub new_records: usize,
    pub names_added: usize,
    pub names_moved: usize,
    pub names_removed: usize,
    pub members_added: usize,
    pub members_removed: usize,
    /// Records collected because no current file reaches them (SR-146).
    pub records_removed: usize,
}

/// What one file of a batch would change (SPEC-010 §5): names added,
/// removed and retargeted, memberships, and records whose span moved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileProposal {
    pub path: String,
    /// Whether the file's text differs from the stored one.
    pub text_changed: bool,
    pub names_added: Vec<(String, Digest)>,
    pub names_removed: Vec<(String, Digest)>,
    /// (name, old target, new target).
    pub names_retargeted: Vec<(String, Digest, Digest)>,
    /// (generic function, member).
    pub members_added: Vec<(String, Digest)>,
    pub members_removed: Vec<(String, Digest)>,
    /// Records whose span is new or moved.
    pub spans_moved: Vec<Digest>,
}

/// What `propose_files` found; nothing was written.
#[derive(Debug, Default)]
pub struct Proposal {
    pub files: Vec<FileProposal>,
    /// Files whose token changed since it was read: (path, expected, found).
    pub conflicts: Vec<(String, u64, u64)>,
    /// What `replace_files` would report, when it would succeed.
    pub report: Option<FileReport>,
    /// The error `replace_files` would raise (`NameClaimed`,
    /// `UnreachedRecord`, `NormaliserChanged`, …), reported, not thrown.
    pub error: Option<StoreError>,
}

/// Where a definition's text is: the file, its source record and the span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSource {
    pub path: String,
    pub source: Digest,
    pub start: u32,
    pub end: u32,
}

impl<B: Backend> Store<B> {
    /// Marks the store as a folder store whose files are the truth: the
    /// interface refuses every edit; only `replace_files` writes content.
    pub fn set_derived(&self) -> Result<()> {
        self.set_meta(MODE_KEY, MODE_DERIVED)
    }

    pub fn is_derived(&self) -> Result<bool> {
        Ok(self.meta(MODE_KEY)?.as_deref() == Some(MODE_DERIVED))
    }

    /// A file's current manifest.
    pub fn file_manifest(&self, path: &str) -> Result<Option<FileManifest>> {
        self.read(|tx| {
            let Some(id) = tx.meta(&file_key(path))? else { return Ok(None) };
            let id = Digest::from_slice(&id).ok_or_else(|| StoreError::Codec("file key".into()))?;
            let r = tx.record(&id)?.ok_or_else(|| StoreError::NotFound(id.to_hex()))?;
            FileManifest::decode(&r.bytes).map(Some)
        })
    }

    /// The paths of all files in the store.
    pub fn files(&self) -> Result<Vec<String>> {
        Ok(self.meta_prefix("file/")?.into_iter().map(|(k, _)| k["file/".len()..].to_string()).collect())
    }

    /// Where a record's text is, if a file contributed it: the file whose
    /// span text contains `name`, else the first.
    pub fn text_source(&self, id: &Digest, name: Option<&str>) -> Result<Option<TextSource>> {
        let paths: Vec<String> =
            self.meta_prefix(&defined_prefix(id))?.into_iter().map(|(k, _)| k[defined_prefix(id).len()..].to_string()).collect();
        let mut found = Vec::new();
        for p in paths {
            let Some(m) = self.file_manifest(&p)? else { continue };
            let Some(source) = m.source else { continue };
            for (rid, a, b) in &m.spans {
                if rid == id {
                    found.push(TextSource { path: p.clone(), source, start: *a, end: *b });
                }
            }
        }
        if let Some(n) = name {
            for (i, t) in found.iter().enumerate() {
                let text = self.record(&t.source)?.and_then(|r| r.bytes.get(t.start as usize..t.end as usize).map(|b| b.to_vec()));
                if text.is_some_and(|b| String::from_utf8_lossy(&b).contains(n)) {
                    return Ok(Some(found.swap_remove(i)));
                }
            }
        }
        Ok(found.into_iter().next())
    }

    /// Replaces everything the given files contributed, in one transaction
    /// (SPEC-009 §2): names in, names out, names moved between files of the
    /// batch, generic-function memberships, spans, facts and the file
    /// manifests. A name owned by a file outside the batch is refused with
    /// [`StoreError::NameClaimed`], and nothing is written.
    pub fn replace_files(&self, files: &[FileIngest]) -> Result<FileReport> {
        let files: Vec<&FileIngest> = files.iter().collect();
        let batch_paths = validate_batch(&files)?;
        self.backend().write(|tx| self.replace_in(tx, &files, &batch_paths))
    }

    /// `replace_files` inside a write transaction, after `validate_batch`.
    fn replace_in(&self, tx: &mut dyn WriteTx, files: &[&FileIngest], batch_paths: &BTreeSet<&str>) -> Result<FileReport> {
        let mut report = FileReport { files: files.len(), ..FileReport::default() };
        // The normaliser of the whole folder.
        if let Some(f) = files.iter().find(|f| f.text.is_some()) {
            match tx.meta(NORMALISER_KEY)? {
                Some(n) if n != f.normaliser.as_bytes() => {
                    return Err(StoreError::NormaliserChanged {
                        stored: String::from_utf8_lossy(&n).into_owned(),
                        new: f.normaliser.clone(),
                    });
                }
                Some(_) => {}
                None => tx.set_meta(NORMALISER_KEY, f.normaliser.as_bytes())?,
            }
        }
        // The batch's old contributions.
        let mut old: BTreeMap<&str, FileManifest> = BTreeMap::new();
        let mut old_manifests = Vec::new();
        for f in files {
            if let Some(id) = tx.meta(&file_key(&f.path))? {
                let id = Digest::from_slice(&id).ok_or_else(|| StoreError::Codec("file key".into()))?;
                old_manifests.push(id);
                let r = tx.record(&id)?.ok_or_else(|| StoreError::NotFound(id.to_hex()))?;
                old.insert(f.path.as_str(), FileManifest::decode(&r.bytes)?);
            }
        }
        // New content names, and names owned outside the batch.
        let mut new_names: BTreeMap<String, (Digest, &str)> = BTreeMap::new();
        for f in files {
            for (n, i) in &f.names {
                new_names.insert(n.clone(), (f.records[*i].id, f.path.as_str()));
            }
        }
        let new_generics: BTreeSet<&str> = files.iter().flat_map(|f| f.generics.iter().map(|g| g.path.as_str())).collect();
        for n in new_names.keys() {
            if let Some(owner) = tx.meta(&owner_key(n))? {
                let owner = String::from_utf8_lossy(&owner).into_owned();
                if !batch_paths.contains(owner.as_str()) {
                    return Err(StoreError::NameClaimed { name: n.clone(), owner });
                }
            } else if let Some(e) = tx.name(n)? {
                // A name no file owns: a generic function's, or not from a
                // file. A generic function's name can become a definition's
                // only when no file outside the batch still contributes.
                let Target::Nominal(g) = e.target else {
                    return Err(StoreError::NameClaimed { name: n.clone(), owner: String::new() });
                };
                let mut outside: HashMap<Digest, i64> = HashMap::new();
                for r in tx.relation(&g)? {
                    let c = tx.meta(&member_key(&g, &r.member))?.map(|b| u32::from_le_bytes(b.try_into().unwrap_or([0; 4]))).unwrap_or(0);
                    outside.insert(r.member, c as i64);
                }
                for m in old.values() {
                    for ((gn, _), ms) in m.contributions() {
                        if gn == *n {
                            for x in ms {
                                *outside.entry(x).or_default() -= 1;
                            }
                        }
                    }
                }
                if outside.values().any(|c| *c > 0) {
                    return Err(StoreError::NameClaimed { name: n.clone(), owner: String::new() });
                }
            }
        }
        for n in &new_generics {
            if let Some(owner) = tx.meta(&owner_key(n))? {
                let owner = String::from_utf8_lossy(&owner).into_owned();
                let moving = old.get(owner.as_str()).is_some_and(|m| m.names.iter().any(|(x, _)| x == n));
                if !(batch_paths.contains(owner.as_str()) && moving && !new_names.contains_key(*n)) {
                    return Err(StoreError::NameClaimed { name: n.to_string(), owner });
                }
            } else if let Some(e) = tx.name(n)? {
                if !matches!(e.target, Target::Nominal(_)) {
                    return Err(StoreError::NameClaimed { name: n.to_string(), owner: String::new() });
                }
            }
        }
        // Records and sources.
        for f in files {
            report.new_records += self.put_new(tx, &f.records)?;
            if let Some(t) = &f.text {
                self.put_new(tx, &[source_record(&f.path, t)])?;
            }
        }
        // Content names: removed, moved, added. A generic function's
        // methods are not counted while its name is a definition's, so a
        // name changing kind has its memberships rebuilt (P-231).
        let mut freed: BTreeSet<String> = BTreeSet::new();
        let mut retired: HashMap<String, NominalId> = HashMap::new();
        for m in old.values() {
            for (n, _) in &m.names {
                if !new_names.contains_key(n) && tx.meta(&owner_key(n))?.is_some_and(|o| o == m.path.as_bytes()) {
                    if let Some(e) = tx.name(n)? {
                        self.log(tx, LogOp::RemoveName, n.clone(), Some(e.target.digest()), None)?;
                        tx.remove_name(n)?;
                        report.names_removed += 1;
                    }
                    tx.remove_meta(&owner_key(n))?;
                    freed.insert(n.clone());
                }
            }
        }
        for (n, (id, path)) in &new_names {
            match tx.name(n)? {
                Some(e) if e.target == Target::Content(*id) => {}
                Some(e) if matches!(e.target, Target::Nominal(_)) => {
                    // A generic function's name becomes a definition's.
                    retired.insert(n.clone(), e.target.digest());
                    self.log(tx, LogOp::RemoveName, n.clone(), Some(e.target.digest()), None)?;
                    tx.remove_name(n)?;
                    add_name(self, tx, n, *id)?;
                    report.names_added += 1;
                }
                Some(e) => {
                    let old_id = e.target.digest();
                    let seq = self.log(tx, LogOp::MoveName, n.clone(), Some(old_id), Some(*id))?;
                    tx.put_name(&NameEntry { target: Target::Content(*id), seq, ..e })?;
                    let lineages = tx.lineages_of(&old_id)?;
                    if lineages.is_empty() {
                        tx.put_lineage(&LineageEntry { lineage: *id, id: *id, seq, supersedes: None })?;
                    }
                    for l in lineages {
                        tx.put_lineage(&LineageEntry { lineage: l, id: *id, seq, supersedes: Some(old_id) })?;
                    }
                    report.names_moved += 1;
                }
                None => {
                    add_name(self, tx, n, *id)?;
                    report.names_added += 1;
                }
            }
            tx.set_meta(&owner_key(n), path.as_bytes())?;
        }
        // Generic functions: memberships counted per file, each member
        // once per file and generic however many definitions share its id.
        let mut deltas: BTreeMap<(String, String), BTreeMap<Digest, i64>> = BTreeMap::new();
        for m in old.values() {
            for ((g, lang), ms) in m.contributions() {
                if freed.contains(&g) {
                    continue; // not counted while the name was a definition's
                }
                let e = deltas.entry((g, lang)).or_default();
                for x in ms {
                    *e.entry(x).or_default() -= 1;
                }
            }
        }
        for f in files {
            for ((g, lang), ms) in f.contributions() {
                let e = deltas.entry((g, lang)).or_default();
                for x in ms {
                    *e.entry(x).or_default() += 1;
                }
            }
        }
        if !freed.is_empty() {
            // Files outside the batch that contribute to a freed name.
            for (k, id) in tx.meta_prefix("file/")? {
                if batch_paths.contains(&k["file/".len()..]) {
                    continue;
                }
                let id = Digest::from_slice(&id).ok_or_else(|| StoreError::Codec("file key".into()))?;
                let r = tx.record(&id)?.ok_or_else(|| StoreError::NotFound(id.to_hex()))?;
                for ((g, lang), ms) in FileManifest::decode(&r.bytes)?.contributions() {
                    if freed.contains(&g) {
                        let e = deltas.entry((g, lang)).or_default();
                        for x in ms {
                            *e.entry(x).or_default() += 1;
                        }
                    }
                }
            }
        }
        for (g, nominal) in &retired {
            // Its methods leave with the generic function.
            let mut n = tx.nominal(nominal)?.ok_or_else(|| StoreError::NotFound(nominal.to_hex()))?;
            for mut e in tx.relation(nominal)? {
                let seq = self.log(tx, LogOp::RemoveMethod, nominal.to_hex(), Some(e.member), None)?;
                e.removed_seq = Some(seq);
                tx.put_relation(&e)?;
                n.version = seq;
                report.members_removed += 1;
            }
            for (k, _) in tx.meta_prefix(&format!("member/{}/", nominal.to_hex()))? {
                tx.remove_meta(&k)?;
            }
            tx.put_nominal(&n)?;
            deltas.retain(|(x, _), _| x != g);
        }
        for ((g, lang), ms) in deltas {
            let nominal = match tx.name(&g)? {
                Some(NameEntry { target: Target::Nominal(id), .. }) => Some(id),
                Some(_) => None, // now a definition's name (handled above)
                None if ms.values().any(|d| *d > 0) => {
                    let id = sophia_hash::merkle::nominal_id(&lang, "generic", &crate::logic::fresh_nonce());
                    let seq = self.log(tx, LogOp::NewNominal, g.clone(), None, Some(id))?;
                    tx.put_nominal(&Nominal { id, lang: lang.clone(), kind: "generic".into(), label: g.clone(), created_seq: seq, version: seq })?;
                    tx.set_meta(&format!("nominal-index/{}", id.to_hex()), b"")?;
                    let seq = self.log(tx, LogOp::AddName, g.clone(), None, Some(id))?;
                    tx.put_name(&NameEntry { path: g.clone(), target: Target::Nominal(id), visibility: Visibility::Public, seq })?;
                    report.names_added += 1;
                    Some(id)
                }
                None => None,
            };
            let Some(nominal) = nominal else { continue };
            let mut n = tx.nominal(&nominal)?.ok_or_else(|| StoreError::NotFound(nominal.to_hex()))?;
            let mut relation: HashMap<Digest, RelationEntry> = tx.relation(&nominal)?.into_iter().map(|e| (e.member, e)).collect();
            for (member, delta) in ms {
                if delta == 0 {
                    continue;
                }
                let key = member_key(&nominal, &member);
                let before = tx.meta(&key)?.map(|b| u32::from_le_bytes(b.try_into().unwrap_or([0; 4]))).unwrap_or(0) as i64;
                let after = (before + delta).max(0);
                if after == 0 {
                    tx.remove_meta(&key)?;
                } else {
                    tx.set_meta(&key, &(after as u32).to_le_bytes())?;
                }
                if before == 0 && after > 0 && !relation.contains_key(&member) {
                    let seq = self.log(tx, LogOp::AddMethod, nominal.to_hex(), None, Some(member))?;
                    let e = RelationEntry { nominal, member, added_seq: seq, removed_seq: None };
                    tx.put_relation(&e)?;
                    relation.insert(member, e);
                    n.version = seq;
                    report.members_added += 1;
                } else if before > 0 && after == 0 {
                    if let Some(mut e) = relation.remove(&member) {
                        let seq = self.log(tx, LogOp::RemoveMethod, nominal.to_hex(), Some(member), None)?;
                        e.removed_seq = Some(seq);
                        tx.put_relation(&e)?;
                        n.version = seq;
                        report.members_removed += 1;
                    }
                }
            }
            tx.put_nominal(&n)?;
            // A generic function lives while some file contributes a method.
            if relation.is_empty() && tx.meta(&owner_key(&g))?.is_none() {
                if let Some(e) = tx.name(&g)?.filter(|e| e.target == Target::Nominal(nominal)) {
                    self.log(tx, LogOp::RemoveName, g.clone(), Some(e.target.digest()), None)?;
                    tx.remove_name(&g)?;
                    report.names_removed += 1;
                }
            }
        }
        // Facts: counted per contributing file, like memberships. A fact
        // goes when its last file drops it, unless it was stored before
        // any file contributed it.
        let mut fact_delta: BTreeMap<Digest, i64> = BTreeMap::new();
        let mut new_facts: HashMap<Digest, &Fact> = HashMap::new();
        for m in old.values() {
            for id in &m.facts {
                *fact_delta.entry(*id).or_default() -= 1;
            }
        }
        for f in files {
            let mine: BTreeSet<Digest> = f.facts.iter().map(|x| x.id).collect();
            for x in &f.facts {
                new_facts.insert(x.id, x);
            }
            for id in mine {
                *fact_delta.entry(id).or_default() += 1;
            }
        }
        for (id, delta) in fact_delta {
            if delta == 0 {
                continue;
            }
            let key = fact_key(&id);
            let before = tx.meta(&key)?.map(|b| u32::from_le_bytes(b.try_into().unwrap_or([0; 4]))).unwrap_or(0) as i64;
            let after = (before + delta).max(0);
            if after == 0 {
                tx.remove_meta(&key)?;
            } else {
                tx.set_meta(&key, &(after as u32).to_le_bytes())?;
            }
            if before == 0 && after > 0 {
                let Some(fact) = new_facts.get(&id) else { continue };
                if tx.facts_of(&fact.subject)?.iter().any(|g| g.id == id) {
                    tx.set_meta(&fact_external_key(&id), b"")?;
                } else {
                    self.log(tx, LogOp::Fact, fact.kind.clone(), None, Some(id))?;
                    tx.put_fact(fact)?;
                    report.facts_added += 1;
                }
            } else if before > 0 && after == 0 {
                if tx.meta(&fact_external_key(&id))?.is_some() {
                    tx.remove_meta(&fact_external_key(&id))?;
                } else {
                    // Logged as a fact entry with only an old id.
                    self.log(tx, LogOp::Fact, "removed".into(), Some(id), None)?;
                    tx.remove_fact(&id)?;
                    report.facts_removed += 1;
                }
            }
        }
        // Manifests and spans.
        for f in files {
            if let Some(m) = old.get(f.path.as_str()) {
                for (id, _, _) in &m.spans {
                    tx.remove_meta(&format!("{}{}", defined_prefix(id), f.path))?;
                }
            }
            if f.text.is_none() && f.records.is_empty() {
                tx.remove_meta(&file_key(&f.path))?;
                continue;
            }
            let m = FileManifest {
                path: f.path.clone(),
                normaliser: f.normaliser.clone(),
                source: f.text.as_ref().map(|t| source_record(&f.path, t).id),
                names: f.names.iter().map(|(n, i)| (n.clone(), f.records[*i].id)).collect(),
                generics: f
                    .generics
                    .iter()
                    .map(|g| (g.path.clone(), g.lang.clone(), g.members.iter().map(|i| f.records[*i].id).collect()))
                    .collect(),
                spans: f.spans.iter().map(|(i, a, b)| (f.records[*i].id, *a, *b)).collect(),
                facts: f.facts.iter().map(|x| x.id).collect::<BTreeSet<_>>().into_iter().collect(),
            };
            let r = m.record();
            self.put_new(tx, std::slice::from_ref(&r))?;
            tx.set_meta(&file_key(&f.path), r.id.as_bytes())?;
            for (id, _, _) in &m.spans {
                tx.set_meta(&format!("{}{}", defined_prefix(id), f.path), b"")?;
            }
        }
        // When the files are the truth, what they no longer reach goes;
        // git keeps the history (SR-146).
        if tx.meta(MODE_KEY)?.as_deref() == Some(MODE_DERIVED) {
            report.records_removed = self.collect(tx, old_manifests)?;
        }
        Ok(report)
    }

    /// A file's token (SPEC-010 §2): a hash over its current manifest and
    /// the state of every name and generic function the manifest lists. It
    /// changes when the file is replaced or one of its names moves. 0 for a
    /// file the store does not have.
    pub fn file_token(&self, path: &str) -> Result<u64> {
        self.read(|tx| file_token_in(tx, path))
    }

    /// The file that lays out `name`, and its token: names a file lists are
    /// edited through the file in a store-truth folder (SPEC-010 §2).
    pub fn laid_out(&self, name: &str) -> Result<Option<(String, u64)>> {
        self.read(|tx| {
            let Some(owner) = tx.meta(&owner_key(name))? else { return Ok(None) };
            let path = String::from_utf8_lossy(&owner).into_owned();
            let token = file_token_in(tx, &path)?;
            Ok(Some((path, token)))
        })
    }

    /// `replace_files` for a store-truth folder (SPEC-010 §2): each file
    /// carries the token read when it was last printed or read. If any
    /// token changed, nothing is written ([`StoreError::FileConflict`]).
    /// Refused on a derived store, whose files are the truth.
    pub fn replace_files_checked(&self, files: &[(FileIngest, u64)]) -> Result<FileReport> {
        let ingests: Vec<&FileIngest> = files.iter().map(|(f, _)| f).collect();
        let batch_paths = validate_batch(&ingests)?;
        self.backend().write(|tx| {
            if tx.meta(MODE_KEY)?.as_deref() == Some(MODE_DERIVED) {
                return Err(StoreError::Derived);
            }
            let conflicts = conflicts_in(tx, files)?;
            if !conflicts.is_empty() {
                return Err(StoreError::FileConflict { conflicts });
            }
            self.replace_in(tx, &ingests, &batch_paths)
        })
    }

    /// What `replace_files_checked` would do, without writing (SPEC-010
    /// §5): per file the names, memberships and spans that change, the
    /// conflicting tokens, and the report or error. It runs the
    /// replacement in a write transaction that is always rolled back, so
    /// the answer is exactly what an accept would do now.
    pub fn propose_files(&self, files: &[(FileIngest, u64)]) -> Result<Proposal> {
        let ingests: Vec<&FileIngest> = files.iter().map(|(f, _)| f).collect();
        let mut proposal = Proposal::default();
        let batch_paths = match validate_batch(&ingests) {
            Ok(p) => p,
            Err(e) => {
                proposal.error = Some(e);
                return Ok(proposal);
            }
        };
        let mut outcome: Option<Result<Proposal>> = None;
        let rolled_back = self.backend().write(|tx| {
            let r = (|| {
                if tx.meta(MODE_KEY)?.as_deref() == Some(MODE_DERIVED) {
                    return Err(StoreError::Derived);
                }
                let mut p = Proposal { conflicts: conflicts_in(tx, files)?, ..Proposal::default() };
                for f in &ingests {
                    p.files.push(file_proposal(tx, f)?);
                }
                match self.replace_in(tx, &ingests, &batch_paths) {
                    Ok(r) => p.report = Some(r),
                    Err(StoreError::Backend(e)) => return Err(StoreError::Backend(e)),
                    Err(e) => p.error = Some(e),
                }
                Ok(p)
            })();
            outcome = Some(r);
            Err::<(), _>(StoreError::Backend("propose_files: rolled back".into()))
        });
        match outcome {
            Some(r) => r,
            None => Err(rolled_back.err().unwrap_or_else(|| StoreError::Backend("propose_files".into()))),
        }
    }

    /// Moves the truth from the files into the store (SPEC-010 §3), in
    /// place: replaces `files` (the folder's current files, normally the
    /// stale ones), then ends the derived mode, in one transaction. From
    /// then on nothing is collected; history before it stays in git.
    pub fn take_truth(&self, files: &[FileIngest]) -> Result<FileReport> {
        let ingests: Vec<&FileIngest> = files.iter().collect();
        let batch_paths = validate_batch(&ingests)?;
        self.backend().write(|tx| {
            if tx.meta(MODE_KEY)?.as_deref() != Some(MODE_DERIVED) {
                return Err(StoreError::NotDerived);
            }
            let report = self.replace_in(tx, &ingests, &batch_paths)?;
            tx.remove_meta(MODE_KEY)?;
            let seq = self.log(tx, LogOp::Setting, "store/truth=store".into(), None, None)?;
            tx.set_meta(TRUTH_SINCE_KEY, seq.to_string().as_bytes())?;
            Ok(report)
        })
    }

    /// Gives the truth back to the files (SPEC-010 §3): `exported` is every
    /// file Moonkale wrote, as (path, source id of the text written). Each
    /// must be a current file with exactly that stored text, and every
    /// current file must be listed ([`StoreError::ExportMismatch`]). Then
    /// the store is derived again and collected, in one transaction: **the
    /// store's version history is discarded**. Returns the records removed.
    pub fn give_truth(&self, exported: &[(String, Digest)]) -> Result<usize> {
        self.backend().write(|tx| {
            if tx.meta(MODE_KEY)?.as_deref() == Some(MODE_DERIVED) {
                return Err(StoreError::Derived);
            }
            let given: BTreeMap<&str, &Digest> = exported.iter().map(|(p, d)| (p.as_str(), d)).collect();
            let current = tx.meta_prefix("file/")?;
            for (k, id) in &current {
                let path = &k["file/".len()..];
                let id = Digest::from_slice(id).ok_or_else(|| StoreError::Codec("file key".into()))?;
                let r = tx.record(&id)?.ok_or_else(|| StoreError::NotFound(id.to_hex()))?;
                let source = FileManifest::decode(&r.bytes)?.source;
                if source.is_none() || given.get(path).copied() != source.as_ref() {
                    return Err(StoreError::ExportMismatch { path: path.to_string() });
                }
            }
            if let Some(p) = given.keys().find(|p| !current.iter().any(|(k, _)| &k["file/".len()..] == **p)) {
                return Err(StoreError::ExportMismatch { path: p.to_string() });
            }
            tx.set_meta(MODE_KEY, MODE_DERIVED)?;
            tx.remove_meta(TRUTH_SINCE_KEY)?;
            self.log(tx, LogOp::Setting, "store/truth=files".into(), None, None)?;
            let all = tx.records_of_kind(None, usize::MAX)?;
            self.collect(tx, all)
        })
    }

    /// Collects every record of a derived store that no current file,
    /// name or generic member reaches (SR-146), for stores written
    /// before collection. Refused when the store is not derived: there the
    /// store is the truth and its history is kept.
    pub fn collect_folder(&self) -> Result<usize> {
        self.backend().write(|tx| {
            if tx.meta(MODE_KEY)?.as_deref() != Some(MODE_DERIVED) {
                return Err(StoreError::NotDerived);
            }
            let all = tx.records_of_kind(None, usize::MAX)?;
            self.collect(tx, all)
        })
    }

    /// Removes the records among `seeds` that nothing reaches, then what
    /// only they reached. A component goes as a whole. A record is kept
    /// while it is a current file manifest, a name's target, a current
    /// generic member, or referenced by a stored record outside its
    /// component; removing a referrer checks its references again. Facts do
    /// not keep records: the files are the truth, so the store holds what a
    /// fresh build of them holds, and a fact about a record no file has any
    /// more dangles there too.
    fn collect(&self, tx: &mut dyn WriteTx, seeds: Vec<Digest>) -> Result<usize> {
        let mut stack = seeds;
        let mut removed = 0;
        while let Some(x) = stack.pop() {
            let Some(r) = tx.record(&x)? else { continue };
            let group = if r.component_size > 1 { tx.component(&r.component)? } else { vec![r] };
            let ids: BTreeSet<Digest> = group.iter().map(|m| m.id).collect();
            let mut reached = false;
            for m in &group {
                reached = (m.lang == MANIFEST_LANG && m.kind == MANIFEST_KIND && current_manifest(tx, m)?)
                    || !tx.names_of(&m.id)?.is_empty()
                    || !tx.memberships_of(&m.id)?.is_empty()
                    || tx.dependants(&m.id)?.iter().any(|d| !ids.contains(d));
                if reached {
                    break;
                }
            }
            if reached {
                continue;
            }
            for m in group {
                self.log(tx, LogOp::RemoveDefinition, m.id.to_hex(), Some(m.id), None)?;
                tx.remove_record(&m.id)?;
                removed += 1;
                stack.extend(m.refs.iter().filter(|d| !ids.contains(d)));
            }
        }
        Ok(removed)
    }
}

/// Checks a batch before any transaction: paths once each, indices in
/// range, every record reached, a name claimed by one file. Returns the
/// batch's paths.
fn validate_batch<'a>(files: &[&'a FileIngest]) -> Result<BTreeSet<&'a str>> {
    let mut batch_paths = BTreeSet::new();
    for f in files {
        if !batch_paths.insert(f.path.as_str()) {
            return Err(StoreError::NotFound(format!("file {} twice in one batch", f.path)));
        }
        let n = f.records.len();
        let bad = |i: usize| StoreError::NotFound(format!("{}: record index {i}", f.path));
        for (name, i) in &f.names {
            validate_path(name)?;
            if *i >= n {
                return Err(bad(*i));
            }
        }
        for g in &f.generics {
            validate_path(&g.path)?;
            if let Some(i) = g.members.iter().find(|i| **i >= n) {
                return Err(bad(*i));
            }
        }
        for (i, a, b) in &f.spans {
            let len = f.text.as_ref().map_or(0, String::len) as u32;
            if *i >= n || a > b || *b > len {
                return Err(StoreError::NotFound(format!("{}: span {a}..{b} of record {i}", f.path)));
            }
        }
        // Every record must be reached from what the manifest references
        // (names, members, spans), else collection never sees it (RQ-002).
        let mut reached: HashSet<Digest> = HashSet::new();
        let mut stack: Vec<Digest> = f.names.iter().map(|(_, i)| f.records[*i].id).collect();
        stack.extend(f.generics.iter().flat_map(|g| g.members.iter().map(|i| f.records[*i].id)));
        stack.extend(f.spans.iter().map(|(i, _, _)| f.records[*i].id));
        let by_id: HashMap<Digest, &Record> = f.records.iter().map(|r| (r.id, r)).collect();
        while let Some(d) = stack.pop() {
            if reached.insert(d) {
                if let Some(r) = by_id.get(&d) {
                    stack.extend(r.refs.iter().copied());
                }
            }
        }
        if let Some(r) = f.records.iter().find(|r| !reached.contains(&r.id)) {
            return Err(StoreError::UnreachedRecord { path: f.path.clone(), id: r.id.to_hex() });
        }
    }
    // Claims within the batch: a name belongs to one file.
    let mut claims: HashMap<&str, &str> = HashMap::new();
    for f in files {
        let mine: BTreeSet<&str> =
            f.names.iter().map(|(n, _)| n.as_str()).chain(f.generics.iter().map(|g| g.path.as_str())).collect();
        for n in mine {
            let generic = f.generics.iter().any(|g| g.path == n);
            if let Some(other) = claims.get(n) {
                // Generic functions are shared; content names are not.
                let other_generic = files.iter().find(|x| x.path == *other).is_some_and(|x| x.generics.iter().any(|g| g.path == n));
                if !(generic && other_generic) {
                    return Err(StoreError::NameClaimed { name: n.to_string(), owner: other.to_string() });
                }
            }
            claims.insert(n, f.path.as_str());
        }
    }
    Ok(batch_paths)
}

/// See [`Store::file_token`].
fn file_token_in<T: ReadTx + ?Sized>(tx: &T, path: &str) -> Result<u64> {
    let Some(id) = tx.meta(&file_key(path))? else { return Ok(0) };
    let id = Digest::from_slice(&id).ok_or_else(|| StoreError::Codec("file key".into()))?;
    let r = tx.record(&id)?.ok_or_else(|| StoreError::NotFound(id.to_hex()))?;
    let m = FileManifest::decode(&r.bytes)?;
    let mut h = blake3::Hasher::new();
    h.update(b"sophia-file-token-v1");
    h.update(id.as_bytes());
    let mut names: Vec<&str> = m.names.iter().map(|(n, _)| n.as_str()).collect();
    names.extend(m.generics.iter().map(|(g, _, _)| g.as_str()));
    names.sort_unstable();
    names.dedup();
    for n in names {
        h.update(n.as_bytes());
        h.update(&[0]);
        match tx.name(n)? {
            Some(e) => {
                h.update(&e.seq.to_le_bytes());
                h.update(e.target.digest().as_bytes());
                if let Target::Nominal(g) = e.target {
                    if let Some(nom) = tx.nominal(&g)? {
                        h.update(&nom.version.to_le_bytes());
                    }
                }
            }
            None => {
                h.update(b"absent");
            }
        }
    }
    let t = u64::from_le_bytes(h.finalize().as_bytes()[..8].try_into().expect("8 bytes"));
    Ok(if t == 0 { 1 } else { t })
}

/// The files whose token differs from the one given.
fn conflicts_in(tx: &dyn WriteTx, files: &[(FileIngest, u64)]) -> Result<Vec<(String, u64, u64)>> {
    let mut out = Vec::new();
    for (f, expected) in files {
        let found = file_token_in(tx, &f.path)?;
        if found != *expected {
            out.push((f.path.clone(), *expected, found));
        }
    }
    Ok(out)
}

/// One file's changes against its stored manifest.
fn file_proposal(tx: &dyn WriteTx, f: &FileIngest) -> Result<FileProposal> {
    let old = match tx.meta(&file_key(&f.path))? {
        Some(id) => {
            let id = Digest::from_slice(&id).ok_or_else(|| StoreError::Codec("file key".into()))?;
            let r = tx.record(&id)?.ok_or_else(|| StoreError::NotFound(id.to_hex()))?;
            FileManifest::decode(&r.bytes)?
        }
        None => FileManifest::default(),
    };
    let mut p = FileProposal {
        path: f.path.clone(),
        text_changed: old.source != f.text.as_ref().map(|t| source_record(&f.path, t).id),
        ..FileProposal::default()
    };
    let old_names: BTreeMap<&str, Digest> = old.names.iter().map(|(n, d)| (n.as_str(), *d)).collect();
    let new_names: BTreeMap<&str, Digest> = f.names.iter().map(|(n, i)| (n.as_str(), f.records[*i].id)).collect();
    for (n, d) in &new_names {
        match old_names.get(n) {
            None => p.names_added.push((n.to_string(), *d)),
            Some(o) if o != d => p.names_retargeted.push((n.to_string(), *o, *d)),
            Some(_) => {}
        }
    }
    for (n, d) in &old_names {
        if !new_names.contains_key(n) {
            p.names_removed.push((n.to_string(), *d));
        }
    }
    let old_c = old.contributions();
    let new_c = f.contributions();
    for ((g, _), ms) in &new_c {
        let before = old_c.iter().find(|((x, _), _)| x == g).map(|(_, m)| m);
        for m in ms {
            if !before.is_some_and(|b| b.contains(m)) {
                p.members_added.push((g.clone(), *m));
            }
        }
    }
    for ((g, _), ms) in &old_c {
        let after = new_c.iter().find(|((x, _), _)| x == g).map(|(_, m)| m);
        for m in ms {
            if !after.is_some_and(|a| a.contains(m)) {
                p.members_removed.push((g.clone(), *m));
            }
        }
    }
    let old_spans: BTreeSet<(Digest, u32, u32)> = old.spans.iter().copied().collect();
    let mut moved: BTreeSet<Digest> = BTreeSet::new();
    for (i, a, b) in &f.spans {
        let id = f.records[*i].id;
        if !old_spans.contains(&(id, *a, *b)) {
            moved.insert(id);
        }
    }
    p.spans_moved = moved.into_iter().collect();
    Ok(p)
}

/// Whether a manifest record is some file's current manifest.
fn current_manifest(tx: &dyn WriteTx, r: &Record) -> Result<bool> {
    let m = FileManifest::decode(&r.bytes)?;
    Ok(tx.meta(&file_key(&m.path))?.as_deref() == Some(r.id.as_bytes().as_slice()))
}

fn add_name<B: Backend>(store: &Store<B>, tx: &mut dyn WriteTx, n: &str, id: Digest) -> Result<()> {
    let seq = store.log(tx, LogOp::AddName, n.to_string(), None, Some(id))?;
    tx.put_name(&NameEntry { path: n.to_string(), target: Target::Content(id), visibility: Visibility::Public, seq })?;
    if tx.lineages_of(&id)?.is_empty() {
        tx.put_lineage(&LineageEntry { lineage: id, id, seq, supersedes: None })?;
    }
    Ok(())
}
