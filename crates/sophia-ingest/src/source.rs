//! Source files as content-addressed records, and the **ingest manifest**
//! that says how records were derived from them. With both stored, every
//! record an ingest produced can be rebuilt from the store alone and checked
//! field by field ([`reconstruct`]). That is the precondition for ever
//! dropping derived bytes ([[Current Inventory]], [[Materialization Policy]]).
//! See `source.md`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use sophia_hash::canonical::{Reader, Writer};
use sophia_hash::merkle::{component_digest, member_digest};
use sophia_hash::{Digest, Record, Status};
use sophia_store::{Backend, Store, StoreError, Visibility};

use crate::{metamath, module};

/// Source files: language `source`, kind `source-file`, bytes = the file's
/// bytes exactly. The id covers the content only; the path is in the
/// manifest (one text at two paths is one record).
pub const SOURCE_LANG: &str = "source";
pub const SOURCE_NORMALISER: &str = "sophia-source@0";
pub const MANIFEST_LANG: &str = "sophia-ingest";
pub const MANIFEST_NORMALISER: &str = "sophia-ingest-manifest@0";

/// The frontends a manifest can name, with their version: a frontend change
/// that changes what it produces bumps it.
pub const JULIA_MODULE_FRONTEND: &str = "sophia-ingest/julia-module@0";
pub const METAMATH_FRONTEND: &str = "sophia-ingest/metamath@0";

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

/// A source file's record.
pub fn source_record(path: &str, text: &str) -> Record {
    single(SOURCE_LANG, SOURCE_NORMALISER, "source-file", text.as_bytes().to_vec(), path.as_bytes().to_vec(), Vec::new())
}

/// One file of an ingest, in the order the frontend read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestFile {
    /// Relative to the package's source directory.
    pub path: String,
    pub source: Digest,
    /// The module path the file was included at (Julia; empty otherwise).
    pub base_module: Vec<String>,
}

/// How an ingest derived records from sources: everything that, with the
/// source records, determines the output. A transformation record in the
/// sense of SPEC-006 §6a.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub frontend: String,
    /// The normaliser whose ids the records carry (`moonkale-wp66@7`).
    pub normaliser: String,
    pub package: String,
    pub files: Vec<ManifestFile>,
    /// Julia: (file index, module path) of each dynamic include.
    pub dynamic: Vec<(u32, Vec<String>)>,
}

impl Manifest {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.str("frontend", &self.frontend);
        w.str("normaliser", &self.normaliser);
        w.str("package", &self.package);
        for f in &self.files {
            let o = w.open("file");
            w.str("path", &f.path);
            w.field("source", f.source.as_bytes());
            for m in &f.base_module {
                w.str("module", m);
            }
            w.close(o);
        }
        for (i, m) in &self.dynamic {
            let o = w.open("dynamic");
            w.u32("file", *i);
            for s in m {
                w.str("module", s);
            }
            w.close(o);
        }
        w.finish()
    }

    pub fn decode(bytes: &[u8]) -> Result<Manifest, String> {
        let e = |x: sophia_hash::canonical::ReadError| format!("manifest: {x:?}");
        let mut r = Reader::new(bytes);
        let mut m = Manifest {
            frontend: r.expect_str("frontend").map_err(e)?.into(),
            normaliser: r.expect_str("normaliser").map_err(e)?.into(),
            package: r.expect_str("package").map_err(e)?.into(),
            files: Vec::new(),
            dynamic: Vec::new(),
        };
        while !r.is_empty() {
            match r.peek_tag().map_err(e)? {
                "file" => {
                    let mut f = r.enter("file").map_err(e)?;
                    let path = f.expect_str("path").map_err(e)?.to_string();
                    let source = Digest::from_slice(f.expect("source").map_err(e)?).ok_or("manifest: bad digest")?;
                    let mut base_module = Vec::new();
                    while !f.is_empty() {
                        base_module.push(f.expect_str("module").map_err(e)?.to_string());
                    }
                    m.files.push(ManifestFile { path, source, base_module });
                }
                "dynamic" => {
                    let mut d = r.enter("dynamic").map_err(e)?;
                    let i = d.expect_u32("file").map_err(e)?;
                    let mut path = Vec::new();
                    while !d.is_empty() {
                        path.push(d.expect_str("module").map_err(e)?.to_string());
                    }
                    m.dynamic.push((i, path));
                }
                t => return Err(format!("manifest: unexpected field {t}")),
            }
        }
        Ok(m)
    }

    /// The manifest's record. Its references are the source records, so a
    /// manifest keeps its sources reachable ([[Garbage Collection]]).
    pub fn record(&self) -> Record {
        let mut refs: Vec<Digest> = self.files.iter().map(|f| f.source).collect();
        refs.sort();
        refs.dedup();
        single(MANIFEST_LANG, MANIFEST_NORMALISER, "ingest-manifest", self.encode(), self.package.as_bytes().to_vec(), refs)
    }
}

/// The name of a package's current manifest: `ingest.<prefix>.<package>`.
pub fn manifest_name(prefix: &str, package: &str) -> String {
    let plain = package.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && package.chars().all(|c| c.is_alphanumeric() || c == '_');
    if plain { format!("ingest.{prefix}.{package}") } else { format!("ingest.{prefix}.`{package}`") }
}

/// Stores the sources and the manifest, and points `name` at the manifest.
/// Returns the manifest's id.
pub fn store_manifest<B: Backend>(
    store: &Store<B>,
    name: &str,
    manifest: &Manifest,
    texts: &[&str],
) -> Result<Digest, StoreError> {
    let mut records: Vec<Record> = manifest.files.iter().zip(texts).map(|(f, t)| source_record(&f.path, t)).collect();
    let m = manifest.record();
    let id = m.id;
    records.push(m);
    let at = records.len() - 1;
    match store.resolve(name)? {
        None => {
            store.add(&records, &[(name.to_string(), at, Visibility::Public)])?;
        }
        Some(e) if e.target.digest() == id => {
            store.put_records(&records)?;
        }
        Some(e) => {
            store.update(name, &records, at, e.seq)?;
        }
    }
    Ok(id)
}

/// The result of rebuilding an ingest from the store.
#[derive(Debug, Clone, Default)]
pub struct Reconstruction {
    pub frontend: String,
    pub sources: usize,
    pub source_bytes: usize,
    /// Records the frontend produced from the stored sources.
    pub rebuilt: usize,
    /// Their ids, sorted.
    pub ids: Vec<Digest>,
    /// Of those, stored and identical in every content field.
    pub identical: usize,
    /// Rebuilt but not in the store.
    pub missing: Vec<Digest>,
    /// Stored under the same id with a different field: (id, field).
    pub differing: Vec<(Digest, String)>,
    pub time: Duration,
}

impl Reconstruction {
    pub fn ok(&self) -> bool {
        self.rebuilt > 0 && self.identical == self.rebuilt && self.missing.is_empty() && self.differing.is_empty()
    }
}

/// The first content field in which two records differ. `nominal_refs` is
/// not compared: it binds to nominal entities, which are store state.
pub fn first_difference(stored: &Record, rebuilt: &Record) -> Option<&'static str> {
    let checks: [(&'static str, bool); 11] = [
        ("id", stored.id == rebuilt.id),
        ("lang", stored.lang == rebuilt.lang),
        ("normaliser", stored.normaliser == rebuilt.normaliser),
        ("kind", stored.kind == rebuilt.kind),
        ("component", stored.component == rebuilt.component),
        ("index", stored.index == rebuilt.index),
        ("component_size", stored.component_size == rebuilt.component_size),
        ("bytes", stored.bytes == rebuilt.bytes),
        ("display", stored.display == rebuilt.display),
        ("status", stored.status == rebuilt.status),
        ("refs", stored.refs == rebuilt.refs),
    ];
    checks.iter().find(|(_, same)| !same).map(|(f, _)| *f)
}

/// Rebuilds every record of the ingest that `manifest` describes, from the
/// store alone (the manifest and its source records), with the frontend
/// compiled into this binary, and compares each with the stored record.
/// A manifest naming another normaliser than this binary's cannot be
/// rebuilt here, and says so.
pub fn reconstruct<B: Backend>(store: &Store<B>, manifest: &Digest) -> Result<Reconstruction, Box<dyn std::error::Error>> {
    let t = Instant::now();
    let mrec = store.record(manifest)?.ok_or("manifest not stored")?;
    let m = Manifest::decode(&mrec.bytes)?;
    let mut texts = Vec::new();
    for f in &m.files {
        let r = store.record(&f.source)?.ok_or_else(|| format!("source {} not stored", f.path))?;
        if r.lang != SOURCE_LANG {
            return Err(format!("{} is not a source record", f.path).into());
        }
        texts.push(String::from_utf8(r.bytes)?);
    }
    let rebuilt: Vec<Record> = match m.frontend.as_str() {
        JULIA_MODULE_FRONTEND => {
            if m.normaliser != module::normaliser() {
                return Err(format!("manifest needs {}, this build has {}", m.normaliser, module::normaliser()).into());
            }
            let d = module::Discovered {
                files: m
                    .files
                    .iter()
                    .zip(&texts)
                    .map(|(f, t)| module::SourceFile { path: f.path.clone(), text: t.clone(), base_module: f.base_module.clone() })
                    .collect(),
                dynamic: m.dynamic.iter().map(|(i, p)| (*i as usize, p.clone())).collect(),
            };
            module::ingest(&d)?.files.into_iter().flat_map(|(_, v)| v.into_iter().map(|(_, r)| r)).collect()
        }
        METAMATH_FRONTEND => {
            if m.normaliser != metamath::NORMALISER {
                return Err(format!("manifest needs {}, this build has {}", m.normaliser, metamath::NORMALISER).into());
            }
            let text = texts.first().ok_or("no source")?;
            metamath::parse(text)?.into_iter().flat_map(|a| std::iter::once(a.record).chain(a.proof)).collect()
        }
        f => return Err(format!("unknown frontend {f}").into()),
    };
    // One record per id (repeated definitions are one record).
    let mut by_id: HashMap<Digest, Record> = HashMap::new();
    for r in rebuilt {
        by_id.entry(r.id).or_insert(r);
    }
    let mut out = Reconstruction {
        frontend: m.frontend.clone(),
        sources: texts.len(),
        source_bytes: texts.iter().map(String::len).sum(),
        rebuilt: by_id.len(),
        ..Reconstruction::default()
    };
    let mut ids: Vec<Digest> = by_id.keys().copied().collect();
    ids.sort();
    out.ids = ids.clone();
    for id in &ids {
        match store.record(id)? {
            None => out.missing.push(*id),
            Some(s) => match first_difference(&s, &by_id[id]) {
                None => out.identical += 1,
                Some(f) => out.differing.push((*id, f.to_string())),
            },
        }
    }
    out.time = t.elapsed();
    Ok(out)
}
