//! The interface Moonkale's `sources/sophia` adapter wraps (SPEC-004). It
//! speaks in its own small types (keys, node and edge views, text with
//! versions, diagnostics) that hold no Moonkale, driver or async types, so
//! the adapter is a thin translation and works with every backend. See
//! `sophia_interface.md`.
//!
//! Mapping to Moonkale ids: `Key::Content(d)` → `NodeId::from_content(d)`;
//! `Key::Derived(s)` → `NodeId::derive(source, s)`.

mod edit;
pub mod profiles;

use std::collections::{BTreeSet, HashSet};

use sophia_hash::{Digest, Record};
use sophia_store::{Backend, ReadTx, Store, StoreError, Target};
use sophialang_core::{Body, definition_of};
use sophialang_syntax::{Env, Found, Profile, ToPrint};

pub use edit::{EditError, Written};
pub use sophialang_syntax::{Diagnostic, Severity, Span};

/// A node's identity, as Moonkale derives it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    /// `NodeId::from_content(digest)`: definitions, nominal entities, annotations.
    Content(Digest),
    /// `NodeId::derive(source, key)`: `ns:<path>`, `name:<path>`, `lineage:<hex>`, `settings`.
    Derived(String),
}

impl Key {
    pub fn ns(path: &str) -> Key {
        Key::Derived(format!("ns:{path}"))
    }
    pub fn name(path: &str) -> Key {
        Key::Derived(format!("name:{path}"))
    }
    pub fn settings() -> Key {
        Key::Derived("settings".into())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Prop {
    Text(String),
    Int(i64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeView {
    pub key: Key,
    /// `Directory`, `Name`, `Definition`, `Annotation`, `sophia:Lineage`, `sophia:Settings`.
    pub kind: String,
    pub label: String,
    pub props: Vec<(String, Prop)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EdgeView {
    pub from: Key,
    pub to: Key,
    /// Shared: `Contains`, `Names`, `References`, `Calls`, `Tests`, `MethodOf`,
    /// `TypeOf`, `Annotates`. Sophia's own: `sophia:Supersedes`, `sophia:InLineage`.
    pub kind: String,
    pub props: Vec<(String, Prop)>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Subgraph {
    pub nodes: Vec<NodeView>,
    pub edges: Vec<EdgeView>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Out,
    In,
    Both,
}

/// What a change in the log affected (SPEC-004 §6).
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub seq: u64,
    pub op: String,
    pub keys: Vec<Key>,
    /// A rendering-profile change: every definition's text may have changed.
    pub reprint_all: bool,
}

/// How risky a text query is, for Moonkale's policy (`Source::classify`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Risk {
    ReadOnly,
    Unsupported,
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// A Sophia store opened for one reader.
pub struct Interface<B: Backend> {
    store: Store<B>,
    reader: String,
}

/// Moonkale's `/` paths ↔ Sophia's dotted paths (SPEC-004 §3).
pub fn path_from_moonkale(p: &str) -> String {
    p.trim_matches('/').replace('/', ".")
}

pub fn path_to_moonkale(p: &str) -> String {
    sophialang_syntax::split_path(p).join("/")
}

fn parent_ns(path: &str) -> &str {
    let segs = sophialang_syntax::split_path(path);
    if segs.len() <= 1 { "" } else { &path[..path.len() - segs.last().unwrap().len() - 1] }
}

fn last_segment(path: &str) -> &str {
    sophialang_syntax::split_path(path).last().copied().unwrap_or(path)
}

impl<B: Backend + Sync> Interface<B> {
    /// Opens a store for a reader (SPEC-004 §4: the reader is given once).
    /// Writes the built-in rendering profiles if they are missing.
    pub fn open(backend: B, reader: impl Into<String>) -> Result<Self> {
        let store = Store::new(backend).with_actor("sophia-interface");
        profiles::ensure_built_ins(&store)?;
        Ok(Interface { store, reader: reader.into() })
    }

    pub fn store(&self) -> &Store<B> {
        &self.store
    }

    pub fn reader(&self) -> &str {
        &self.reader
    }

    pub fn root(&self) -> Key {
        Key::ns("")
    }

    pub fn profile_for(&self, ns: &str, lang: &str) -> Result<Profile> {
        let name = profiles::profile_name_for(&self.store, &self.reader, ns, lang)?;
        profiles::profile(&self.store, &name)
    }

    // ---- nodes ----

    fn name_label(path: &str) -> String {
        last_segment(path).to_string()
    }

    fn definition_view(&self, tx: &dyn ReadTx, r: &Record) -> Result<NodeView> {
        let names = tx.names_of(&r.id)?;
        let label = names.first().map(|n| Self::name_label(&n.path)).unwrap_or_else(|| r.id.short(12));
        let def_kind = match r.kind.as_str() {
            "method" => "function",
            "data" | "layout" => "type",
            other => other,
        };
        Ok(NodeView {
            key: Key::Content(r.id),
            kind: "Definition".into(),
            label,
            props: vec![
                ("def_kind".into(), Prop::Text(def_kind.into())),
                ("sophialang_kind".into(), Prop::Text(r.kind.clone())),
                ("lang".into(), Prop::Text(r.lang.clone())),
                ("frontend".into(), Prop::Text(r.normaliser.split('@').next().unwrap_or("").into())),
                ("frontend_version".into(), Prop::Text(r.normaliser.split('@').nth(1).unwrap_or("").into())),
                ("content_id".into(), Prop::Text(r.id.to_hex())),
                ("component".into(), Prop::Text(r.component.to_hex())),
                ("component_index".into(), Prop::Int(r.index as i64)),
                ("component_size".into(), Prop::Int(r.component_size as i64)),
                ("status".into(), Prop::Text(r.status.as_str().into())),
            ],
        })
    }

    fn nominal_view(&self, tx: &dyn ReadTx, id: &Digest) -> Result<Option<NodeView>> {
        let Some(n) = tx.nominal(id)? else { return Ok(None) };
        let names = tx.names_of(id)?;
        let def_kind = match n.kind.as_str() {
            "generic" => "sophialang:GenericFunction",
            "relation" => "sophialang:relation",
            "abstract" => "interface",
            _ => "type",
        };
        Ok(Some(NodeView {
            key: Key::Content(*id),
            kind: "Definition".into(),
            label: names.first().map(|n| Self::name_label(&n.path)).unwrap_or_else(|| n.label.clone()),
            props: vec![
                ("def_kind".into(), Prop::Text(def_kind.into())),
                ("nominal".into(), Prop::Bool(true)),
                ("lang".into(), Prop::Text(n.lang.clone())),
                ("relation_version".into(), Prop::Int(n.version as i64)),
            ],
        }))
    }

    fn namespace_exists(&self, tx: &dyn ReadTx, path: &str) -> Result<bool> {
        Ok(path.is_empty()
            || tx.meta(&format!("ns/{path}"))?.is_some()
            || !tx.names_under(path, 2)?.iter().all(|n| n.path == path))
    }

    pub fn node(&self, key: &Key) -> Result<Option<NodeView>> {
        self.store.read(|tx| self.node_in(tx, key))
    }

    fn node_in(&self, tx: &dyn ReadTx, key: &Key) -> Result<Option<NodeView>> {
        match key {
            Key::Content(d) => {
                if let Some(r) = tx.record(d)? {
                    return Ok(Some(self.definition_view(tx, &r)?));
                }
                self.nominal_view(tx, d)
            }
            Key::Derived(s) => {
                if let Some(path) = s.strip_prefix("ns:") {
                    if !self.namespace_exists(tx, path)? {
                        return Ok(None);
                    }
                    let label = if path.is_empty() { "/".to_string() } else { Self::name_label(path) };
                    return Ok(Some(NodeView {
                        key: key.clone(),
                        kind: "Directory".into(),
                        label,
                        props: vec![("path".into(), Prop::Text(path.into()))],
                    }));
                }
                if let Some(path) = s.strip_prefix("name:") {
                    let Some(n) = tx.name(path)? else { return Ok(None) };
                    return Ok(Some(NodeView {
                        key: key.clone(),
                        kind: "Name".into(),
                        label: Self::name_label(path),
                        props: vec![
                            ("path".into(), Prop::Text(path.into())),
                            ("visibility".into(), Prop::Text(n.visibility.as_str().into())),
                            ("version".into(), Prop::Int(n.seq as i64)),
                        ],
                    }));
                }
                if let Some(hex) = s.strip_prefix("lineage:") {
                    let Ok(l) = hex.parse::<Digest>() else { return Ok(None) };
                    let entries = tx.lineage(&l)?;
                    if entries.is_empty() {
                        return Ok(None);
                    }
                    return Ok(Some(NodeView {
                        key: key.clone(),
                        kind: "sophia:Lineage".into(),
                        label: format!("lineage {}", l.short(8)),
                        props: vec![("length".into(), Prop::Int(entries.len() as i64))],
                    }));
                }
                if s == "settings" {
                    return Ok(Some(NodeView {
                        key: key.clone(),
                        kind: "sophia:Settings".into(),
                        label: "rendering profile".into(),
                        props: vec![("reader".into(), Prop::Text(self.reader.clone()))],
                    }));
                }
                Ok(None)
            }
        }
    }

    /// Children (`Contains`): a namespace's sub-namespaces and names; the
    /// root also holds the settings node. Definitions have no children by
    /// default (SPEC-004 §2).
    pub fn children(&self, key: &Key) -> Result<Subgraph> {
        self.store.read(|tx| self.children_in(tx, key))
    }

    fn children_in(&self, tx: &dyn ReadTx, key: &Key) -> Result<Subgraph> {
        {
            let mut g = Subgraph::default();
            let Key::Derived(s) = key else { return Ok(g) };
            let Some(ns) = s.strip_prefix("ns:") else { return Ok(g) };
            let mut subs = BTreeSet::new();
            let depth = if ns.is_empty() { 0 } else { sophialang_syntax::split_path(ns).len() };
            for n in tx.names_under(ns, usize::MAX)? {
                let segs = sophialang_syntax::split_path(&n.path);
                if segs.len() == depth + 1 {
                    let k = Key::name(&n.path);
                    if let Some(v) = self.node_in(tx, &k)? {
                        g.nodes.push(v);
                        g.edges.push(EdgeView { from: key.clone(), to: k, kind: "Contains".into(), props: vec![] });
                    }
                } else if segs.len() > depth + 1 {
                    subs.insert(segs[..=depth].join("."));
                }
            }
            for (k, _) in tx.meta_prefix(&if ns.is_empty() { "ns/".to_string() } else { format!("ns/{ns}.") })? {
                let p = &k[3..];
                if sophialang_syntax::split_path(p).len() == depth + 1 {
                    subs.insert(p.to_string());
                }
            }
            for sub in subs {
                let k = Key::ns(&sub);
                if let Some(v) = self.node_in(tx, &k)? {
                    g.nodes.insert(0, v);
                    g.edges.push(EdgeView { from: key.clone(), to: k, kind: "Contains".into(), props: vec![] });
                }
            }
            if ns.is_empty() {
                let k = Key::settings();
                g.nodes.push(self.node_in(tx, &k)?.expect("settings node"));
                g.edges.push(EdgeView { from: key.clone(), to: k, kind: "Contains".into(), props: vec![] });
            }
            Ok(g)
        }
    }

    /// Edges at one node, in a direction.
    fn edges_at(&self, tx: &dyn ReadTx, key: &Key, dir: Direction) -> Result<Vec<EdgeView>> {
        let out = matches!(dir, Direction::Out | Direction::Both);
        let inc = matches!(dir, Direction::In | Direction::Both);
        let mut v = Vec::new();
        let e = |from: Key, to: Key, kind: &str| EdgeView { from, to, kind: kind.into(), props: vec![] };
        match key {
            Key::Derived(s) if s.starts_with("name:") => {
                let path = &s[5..];
                if let Some(n) = tx.name(path)? {
                    if out {
                        v.push(e(key.clone(), Key::Content(n.target.digest()), "Names"));
                    }
                    if inc {
                        v.push(e(Key::ns(parent_ns(path)), key.clone(), "Contains"));
                    }
                }
            }
            Key::Derived(s) if s.starts_with("ns:") => {
                if out {
                    v.extend(self.children_in(tx, key)?.edges);
                }
                if inc && s.len() > 3 {
                    v.push(e(Key::ns(parent_ns(&s[3..])), key.clone(), "Contains"));
                }
            }
            Key::Derived(s) if s.starts_with("lineage:") => {
                if inc {
                    if let Ok(l) = s[8..].parse::<Digest>() {
                        for en in tx.lineage(&l)? {
                            v.push(e(Key::Content(en.id), key.clone(), "sophia:InLineage"));
                        }
                    }
                }
            }
            Key::Content(d) => {
                if let Some(r) = tx.record(d)? {
                    if out {
                        let def = definition_of(&r).ok();
                        for t in &r.refs {
                            v.push(e(key.clone(), Key::Content(*t), "References"));
                        }
                        if r.kind == "test" {
                            for t in &r.refs {
                                v.push(e(key.clone(), Key::Content(*t), "Tests"));
                            }
                        }
                        if let Some(def) = &def {
                            let mut called = HashSet::new();
                            def.walk(&mut |t| {
                                if let sophialang_core::Term::App(h, _) = t {
                                    if let sophialang_core::Term::Ref(c) = h.as_ref() {
                                        called.insert(*c);
                                    }
                                }
                            });
                            for c in called {
                                v.push(e(key.clone(), Key::Content(c), "Calls"));
                            }
                        }
                        for l in tx.lineages_of(d)? {
                            v.push(e(key.clone(), Key::Derived(format!("lineage:{}", l.to_hex())), "sophia:InLineage"));
                            for en in tx.lineage(&l)? {
                                if en.id == *d {
                                    if let Some(old) = en.supersedes {
                                        v.push(e(key.clone(), Key::Content(old), "sophia:Supersedes"));
                                    }
                                }
                            }
                        }
                    }
                    if inc {
                        for src in tx.dependants(d)? {
                            v.push(e(Key::Content(src), key.clone(), "References"));
                        }
                        for n in tx.names_of(d)? {
                            v.push(e(Key::name(&n.path), key.clone(), "Names"));
                        }
                        for a in tx.annotations_of(d)? {
                            v.push(e(Key::Content(a.id), key.clone(), "Annotates"));
                        }
                    }
                } else if inc && tx.nominal(d)?.is_some() {
                    for m in tx.relation(d)? {
                        v.push(e(Key::Content(m.member), key.clone(), "MethodOf"));
                    }
                    for n in tx.names_of(d)? {
                        v.push(e(Key::name(&n.path), key.clone(), "Names"));
                    }
                }
            }
            _ => {}
        }
        Ok(v)
    }

    /// Nodes within `depth` hops, plus the edges among them.
    pub fn neighbours(&self, key: &Key, depth: u32, dir: Direction, limit: usize) -> Result<Subgraph> {
        self.store.read(|tx| {
            let mut g = Subgraph::default();
            let mut seen: HashSet<Key> = HashSet::new();
            let mut frontier = vec![key.clone()];
            seen.insert(key.clone());
            if let Some(n) = self.node_in(tx, key)? {
                g.nodes.push(n);
            }
            for _ in 0..depth {
                let mut next = Vec::new();
                for k in &frontier {
                    for ed in self.edges_at(tx, k, dir)? {
                        let other = if &ed.from == k { ed.to.clone() } else { ed.from.clone() };
                        if seen.insert(other.clone()) {
                            if g.nodes.len() >= limit {
                                g.truncated = true;
                                continue;
                            }
                            if let Some(n) = self.node_in(tx, &other)? {
                                g.nodes.push(n);
                                next.push(other);
                            }
                        }
                        if !g.edges.contains(&ed) {
                            g.edges.push(ed);
                        }
                    }
                }
                frontier = next;
            }
            let present: HashSet<&Key> = g.nodes.iter().map(|n| &n.key).collect();
            g.edges.retain(|e| present.contains(&e.from) && present.contains(&e.to));
            Ok(g)
        })
    }

    /// Everything, capped at `limit` nodes (Moonkale's `Query::All`):
    /// namespaces, names, definitions and named nominal entities, with the
    /// `Contains` and `Names` edges and the dependency edges (`References`,
    /// `Calls`, `Tests`, `MethodOf`, `sophia:Supersedes`, `sophia:InLineage`
    /// among returned nodes). `truncated` is set when the cap cuts.
    pub fn all(&self, limit: usize) -> Result<Subgraph> {
        self.store.read(|tx| {
            let mut g = Subgraph::default();
            let mut keys: Vec<Key> = Vec::new();
            let mut seen: HashSet<Key> = HashSet::new();
            let mut push = |k: Key, keys: &mut Vec<Key>| {
                if seen.insert(k.clone()) {
                    keys.push(k);
                }
            };
            let names = tx.names_under("", usize::MAX)?;
            // Namespaces: every proper prefix of a name, and empty namespaces.
            push(Key::ns(""), &mut keys);
            let mut spaces = BTreeSet::new();
            for n in &names {
                let segs = sophialang_syntax::split_path(&n.path);
                for i in 1..segs.len() {
                    spaces.insert(segs[..i].join("."));
                }
            }
            for (k, _) in tx.meta_prefix("ns/")? {
                spaces.insert(k[3..].to_string());
            }
            for sp in &spaces {
                push(Key::ns(sp), &mut keys);
            }
            for n in &names {
                push(Key::name(&n.path), &mut keys);
            }
            for n in &names {
                push(Key::Content(n.target.digest()), &mut keys);
            }
            for d in tx.records_of_kind(None, usize::MAX)? {
                push(Key::Content(d), &mut keys);
            }
            if keys.len() > limit {
                keys.truncate(limit);
                g.truncated = true;
            }
            for k in &keys {
                if let Some(v) = self.node_in(tx, k)? {
                    g.nodes.push(v);
                }
            }
            let present: HashSet<Key> = g.nodes.iter().map(|n| n.key.clone()).collect();
            // Contains: namespace → sub-namespace and namespace → name.
            for k in &present {
                if let Key::Derived(s) = k {
                    let parent = |p: &str| Key::ns(parent_ns(p));
                    let from = if let Some(p) = s.strip_prefix("ns:").filter(|p| !p.is_empty()) {
                        Some(parent(p))
                    } else {
                        s.strip_prefix("name:").map(parent)
                    };
                    if let Some(from) = from.filter(|f| present.contains(f)) {
                        g.edges.push(EdgeView { from, to: k.clone(), kind: "Contains".into(), props: vec![] });
                    }
                }
            }
            let dependency = ["Names", "References", "Calls", "Tests", "sophia:Supersedes", "sophia:InLineage"];
            for k in &keys {
                if !present.contains(k) {
                    continue;
                }
                let dir = match k {
                    Key::Content(d) if tx.record(d)?.is_none() => Direction::In, // nominal: MethodOf members
                    _ => Direction::Out,
                };
                for e in self.edges_at(tx, k, dir)? {
                    let wanted = dependency.contains(&e.kind.as_str()) || e.kind == "MethodOf";
                    if wanted && present.contains(&e.from) && present.contains(&e.to) && !g.edges.contains(&e) {
                        g.edges.push(e);
                    }
                }
            }
            Ok(g)
        })
    }

    // ---- text ----

    /// The text of a node and its version (SPEC-004 §3): a name prints its
    /// definition (its whole recursive group) in the reader's profile; a bare
    /// definition prints read-only (version 0); the settings node prints the
    /// reader's profile.
    pub fn fetch_text(&self, key: &Key) -> Result<(String, u64)> {
        match key {
            Key::Derived(s) if s == "settings" => self.settings_text(),
            Key::Derived(s) if s.starts_with("name:") => {
                let path = &s[5..];
                let n = self.store.resolve(path)?.ok_or_else(|| StoreError::NoSuchName(path.into()))?;
                Ok((self.print_target(n.target, path)?, n.seq))
            }
            Key::Content(d) => {
                let path = self.store.read(|tx| Ok(tx.names_of(d)?.first().map(|n| n.path.clone())))?;
                let target = if self.store.record(d)?.is_some() { Target::Content(*d) } else { Target::Nominal(*d) };
                let path = path.unwrap_or_else(|| format!("_{}", d.short(8)));
                Ok((self.print_target(target, &path)?, 0))
            }
            _ => Err(StoreError::NotFound(format!("{key:?} has no text"))),
        }
    }

    fn print_target(&self, target: Target, path: &str) -> Result<String> {
        let ns = parent_ns(path);
        let env = StoreEnv { store: &self.store };
        match target {
            Target::Content(d) => {
                let r = self.store.record(&d)?.ok_or_else(|| StoreError::NotFound(d.to_hex()))?;
                let profile = self.profile_for(ns, &r.lang)?;
                let members: Vec<Record> =
                    if r.component_size > 1 { self.store.read(|tx| tx.component(&r.component))? } else { vec![r.clone()] };
                let mut named = Vec::new();
                for m in &members {
                    let name = if m.id == d {
                        last_segment(path).to_string()
                    } else {
                        let names = self.store.read(|tx| tx.names_of(&m.id))?;
                        names.first().map(|n| last_segment(&n.path).to_string()).unwrap_or_else(|| format!("_member{}", m.index))
                    };
                    let def = definition_of(m).map_err(|e| StoreError::Codec(e.to_string()))?;
                    named.push((name, def, m.id, m.index));
                }
                let items: Vec<ToPrint> = named
                    .iter()
                    .map(|(n, def, id, index)| ToPrint { name: n, def, id: Some(*id), index: *index, relation: false })
                    .collect();
                Ok(sophialang_syntax::print(&env, ns, &profile, &items))
            }
            Target::Nominal(id) => {
                let n = self.store.nominal(&id)?.ok_or_else(|| StoreError::NotFound(id.to_hex()))?;
                let profile = self.profile_for(ns, &n.lang)?;
                let members = self.store.methods(&id)?;
                if n.kind == "relation" {
                    for m in &members {
                        let r = self.store.record(m)?.ok_or_else(|| StoreError::NotFound(m.to_hex()))?;
                        if r.kind == "relation" {
                            let def = definition_of(&r).map_err(|e| StoreError::Codec(e.to_string()))?;
                            let items = [ToPrint { name: last_segment(path), def: &def, id: None, index: 0, relation: true }];
                            return Ok(sophialang_syntax::print(&env, ns, &profile, &items));
                        }
                    }
                }
                let mut s = format!("# {} {} (nominal, version {})\n", n.kind, path, n.version);
                for m in members {
                    let names = self.store.read(|tx| tx.names_of(&m))?;
                    s.push_str(&format!("#   member {}\n", names.first().map(|n| n.path.clone()).unwrap_or_else(|| m.short(12))));
                }
                Ok(s)
            }
        }
    }

    // ---- settings ----

    fn personal_profile(&self) -> String {
        format!("reader-{}", self.reader.replace(['/', '.'], "_"))
    }

    fn settings_text(&self) -> Result<(String, u64)> {
        let mine = self.personal_profile();
        let mut own = profiles::own_settings(&self.store, &mine)?;
        let current = profiles::profile_name_for(&self.store, &self.reader, "", "sophialang")?;
        if own.is_empty() {
            own.insert("based_on".into(), current.clone());
        }
        let resolved = profiles::settings(&self.store, &mine)?;
        let mut text = sophialang_syntax::profile::print_settings(&mine, &own);
        text.push_str("# inherited values:\n");
        for (k, v) in resolved.iter().filter(|(k, _)| !own.contains_key(*k)) {
            text.push_str(&format!("#   {k} = \"{v}\"\n"));
        }
        let version = self.settings_version()?;
        Ok((text, version))
    }

    fn settings_version(&self) -> Result<u64> {
        Ok(self
            .store
            .meta(&format!("settings-version/{}", self.reader))?
            .and_then(|v| String::from_utf8(v).ok()?.parse().ok())
            .unwrap_or(0))
    }

    // ---- change feed, queries ----

    /// Log entries after `seq`, with the keys they affected (SPEC-004 §6).
    pub fn changes_since(&self, seq: u64, limit: usize) -> Result<(Vec<Change>, u64, bool)> {
        let c = self.store.changes_since(seq, limit)?;
        let out = c
            .entries
            .iter()
            .map(|e| {
                use sophia_store::LogOp::*;
                let mut keys = Vec::new();
                let mut reprint_all = false;
                match e.op {
                    AddName | MoveName | RemoveName => {
                        keys.push(Key::name(&e.subject));
                        keys.push(Key::ns(parent_ns(&e.subject)));
                    }
                    AddDefinition | Annotate | Fact => keys.extend(e.new.map(Key::Content)),
                    NewNominal => keys.extend(e.new.map(Key::Content)),
                    AddMethod | RemoveMethod => {
                        if let Ok(n) = e.subject.parse::<Digest>() {
                            keys.push(Key::Content(n));
                        }
                        keys.extend(e.new.or(e.old).map(Key::Content));
                    }
                    Setting => {
                        keys.push(Key::settings());
                        reprint_all = true;
                    }
                }
                Change { seq: e.seq, op: e.op.as_str().into(), keys, reprint_all }
            })
            .collect();
        Ok((out, c.next, c.more))
    }

    pub fn classify(dialect: &str) -> Risk {
        if dialect == "sql" { Risk::ReadOnly } else { Risk::Unsupported }
    }
}

#[cfg(feature = "sqlite")]
impl Interface<sophia_store::sqlite::Sqlite> {
    /// A read-only `sql` query (SPEC-004 §5): the table, plus a node for
    /// every id column cell that names a definition or nominal entity.
    pub fn sql(&self, query: &str, limits: sophia_store::query::QueryLimits) -> Result<(sophia_store::query::Table, Subgraph)> {
        let t = self.store.backend().query_readonly(query, limits)?;
        let mut g = Subgraph::default();
        let mut seen = HashSet::new();
        for (i, col) in t.columns.iter().enumerate() {
            if col != "id" && !col.ends_with("_id") {
                continue;
            }
            for row in &t.rows {
                if let sophia_store::query::Value::Text(s) = &row[i] {
                    if let Ok(d) = s.parse::<Digest>() {
                        if seen.insert(d) {
                            if let Some(n) = self.node(&Key::Content(d))? {
                                g.nodes.push(n);
                            }
                        }
                    }
                }
            }
        }
        g.truncated = t.truncated;
        Ok((t, g))
    }
}

/// The store as the syntax layer's environment.
pub(crate) struct StoreEnv<'a, B: Backend> {
    pub store: &'a Store<B>,
}

impl<B: Backend + Sync> Env for StoreEnv<'_, B> {
    fn lookup(&self, path: &str) -> Option<Found> {
        match self.store.resolve(path).ok()??.target {
            Target::Content(d) => Some(Found::Content(d)),
            Target::Nominal(n) => Some(Found::Nominal(n)),
        }
    }

    fn constructors(&self, data: &Digest) -> Option<Vec<String>> {
        let r = self.store.record(data).ok()??;
        match definition_of(&r).ok()?.body {
            Body::Constructors(cs) => Some(cs.into_iter().map(|c| c.name).collect()),
            _ => None,
        }
    }

    fn names_of(&self, target: &Digest) -> Vec<String> {
        self.store.read(|tx| tx.names_of(target)).map(|v| v.into_iter().map(|n| n.path).collect()).unwrap_or_default()
    }
}
