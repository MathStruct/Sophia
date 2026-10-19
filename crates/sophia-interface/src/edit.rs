//! Edits through text (SPEC-004 §3, S-ADR-0006): parse, normalise, store,
//! move names. A parse error stores nothing. See `edit.md`.

use std::collections::{BTreeMap, HashSet};

use sophia_hash::Digest;
use sophia_store::{Backend, StoreError, Target, Visibility};
use sophialang_core::{Body, Definition, Term};
use sophialang_syntax::{Diagnostic, Item, Limits};

use crate::{Interface, Key, StoreEnv, last_segment, parent_ns, path_from_moonkale, profiles};

#[derive(Debug, thiserror::Error)]
pub enum EditError {
    /// The text did not parse; nothing was stored. Ranges are into the text.
    #[error("the text has errors")]
    Diagnostics(Vec<Diagnostic>),
    /// The name changed since the text was fetched.
    #[error("changed since it was read: expected version {expected}, found {found}")]
    Conflict { expected: u64, found: u64 },
    #[error("{0}")]
    Refused(String),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// The outcome of a successful edit.
#[derive(Debug, Clone, PartialEq)]
pub struct Written {
    /// The edited node's new version (for a name: its seq).
    pub version: u64,
    /// What each name in the text now points at.
    pub targets: Vec<(String, Digest)>,
    /// Warnings, such as unresolved names (stored as unresolved).
    pub warnings: Vec<Diagnostic>,
    /// Definitions that still reference content this edit replaced (SP-002).
    pub stale_dependants: Vec<Digest>,
}

type R<T> = Result<T, EditError>;

fn abs(ns: &str, name: &str) -> String {
    if ns.is_empty() { name.to_string() } else { format!("{ns}.{name}") }
}

fn head_relation(def: &Definition) -> Option<Digest> {
    match &def.body {
        Body::Rule { head: Term::App(h, _), .. } => match h.as_ref() {
            Term::Nominal(n) => Some(*n),
            _ => None,
        },
        _ => None,
    }
}

impl<B: Backend + Sync> Interface<B> {
    fn parse(&self, ns: &str, text: &str) -> R<(Vec<Item>, Vec<Diagnostic>)> {
        let env = StoreEnv { store: &self.store };
        sophialang_syntax::parse_text(&env, ns, text, Limits::default()).map_err(EditError::Diagnostics)
    }

    /// Stores parsed items. `edited` is the name being edited and the
    /// version it was read at; `group` the other names that may be updated
    /// (members of the printed recursive group).
    fn apply(&self, ns: &str, items: Vec<Item>, warnings: Vec<Diagnostic>, edited: Option<(&str, u64)>, group: &HashSet<String>) -> R<Written> {
        if let Some((path, expected)) = edited {
            let n = self.store.resolve(path)?.ok_or_else(|| StoreError::NoSuchName(path.into()))?;
            if n.seq != expected {
                return Err(EditError::Conflict { expected, found: n.seq });
            }
        }
        // Validate every name before writing anything.
        for it in &items {
            let p = abs(ns, it.name());
            sophia_store::validate_path(&p)?;
            let is_edited = edited.is_some_and(|(e, _)| e == p);
            if let Some(existing) = self.store.resolve(&p)? {
                let ok = is_edited
                    || group.contains(&p)
                    || matches!((it, existing.target), (Item::Relation { .. }, Target::Nominal(_)));
                if !ok {
                    return Err(EditError::Refused(format!("`{p}` already exists; edit it through its own name")));
                }
            }
        }

        // Relations: existing nominal entities, or new ones.
        let mut nominal_of: BTreeMap<usize, Digest> = BTreeMap::new();
        for (i, it) in items.iter().enumerate() {
            if let Item::Relation { name, .. } = it {
                let p = abs(ns, name);
                let id = match self.store.resolve(&p)?.map(|n| n.target) {
                    Some(Target::Nominal(id)) => id,
                    _ => self.store.new_nominal(sophialang_core::LANG, "relation", &p)?,
                };
                nominal_of.insert(i, id);
            }
        }
        let def_index: Vec<usize> = (0..items.len()).filter(|i| !nominal_of.contains_key(i)).collect();
        let renumber = |t: Term| match t {
            Term::Draft(i) => match nominal_of.get(&(i as usize)) {
                Some(n) => Term::Nominal(*n),
                None => Term::Draft(def_index.iter().position(|&j| j == i as usize).expect("definition item") as u32),
            },
            t => t,
        };
        let mut defs: Vec<Definition> =
            def_index.iter().map(|&i| items[i].definition().clone().map_terms(&mut { renumber })).collect();
        let sigs: Vec<(usize, Definition)> = nominal_of
            .keys()
            .map(|&i| (i, items[i].definition().clone().map_terms(&mut { renumber })))
            .collect();
        let n_defs = defs.len();
        defs.extend(sigs.iter().map(|(_, d)| d.clone()));
        let records = sophialang_core::normalise_batch(&defs, &mut Default::default())
            .map_err(|e| EditError::Refused(e.to_string()))?;

        // Names: new ones are added, existing ones updated.
        let mut new_names = Vec::new();
        let mut updates = Vec::new();
        for (k, &i) in def_index.iter().enumerate() {
            let p = abs(ns, items[i].name());
            match self.store.resolve(&p)? {
                None => new_names.push((p, k, Visibility::Public)),
                Some(n) => updates.push((p, k, n.seq)),
            }
        }
        self.store.add(&records, &new_names)?;
        let mut version = 0;
        let mut stale = Vec::new();
        let mut replaced: Vec<(Digest, Digest)> = Vec::new();
        for (p, k, seq) in updates {
            let expected = match edited {
                Some((e, v)) if e == p => v,
                _ => seq,
            };
            let rep = self.store.update(&p, &records, k, expected).map_err(|e| match e {
                StoreError::Conflict { expected, found, .. } => EditError::Conflict { expected, found },
                e => EditError::Store(e),
            })?;
            if edited.is_some_and(|(e, _)| e == p) {
                version = rep.seq;
            }
            if !rep.unchanged {
                replaced.push((rep.old.digest(), rep.new));
                stale.extend(rep.dependants_of_old);
            }
        }
        // Relation signatures and rules join their relations (S-ADR-0009).
        for (s, (i, _)) in sigs.iter().enumerate() {
            let rel = nominal_of[i];
            let sig = records[n_defs + s].id;
            for m in self.store.methods(&rel)? {
                if m != sig && self.store.record(&m)?.is_some_and(|r| r.kind == "relation") {
                    self.store.remove_method(&rel, &m)?;
                }
            }
            self.store.add_method(&rel, &sig)?;
        }
        for (k, def) in defs.iter().take(n_defs).enumerate() {
            if let Some(rel) = head_relation(def) {
                if self.store.nominal(&rel)?.is_some() {
                    for (old, new) in &replaced {
                        if *new == records[k].id && self.store.methods(&rel)?.contains(old) {
                            self.store.remove_method(&rel, old)?;
                        }
                    }
                    self.store.add_method(&rel, &records[k].id)?;
                }
            }
        }
        if let Some((p, _)) = edited {
            if version == 0 {
                version = self.store.resolve(p)?.map(|n| n.seq).unwrap_or(0);
            }
        }
        let mut targets: Vec<(String, Digest)> =
            def_index.iter().enumerate().map(|(k, &i)| (abs(ns, items[i].name()), records[k].id)).collect();
        targets.extend(nominal_of.iter().map(|(&i, id)| (abs(ns, items[i].name()), *id)));
        stale.sort();
        stale.dedup();
        let ours: HashSet<Digest> = records.iter().map(|r| r.id).collect();
        stale.retain(|d| !ours.contains(d));
        Ok(Written { version, targets, warnings, stale_dependants: stale })
    }

    /// Saves edited text (SPEC-004 §3). For a name: the text is parsed in
    /// the name's namespace and must define that name; other names of its
    /// recursive group may be updated too, and new names may be added. For
    /// the settings node: the reader's profile is replaced.
    pub fn write_text(&self, key: &Key, expected: u64, text: &str) -> R<Written> {
        match key {
            Key::Derived(s) if s == "settings" => self.write_settings(expected, text),
            Key::Derived(s) if s.starts_with("name:") => {
                let path = s[5..].to_string();
                let ns = parent_ns(&path).to_string();
                let (items, warnings) = self.parse(&ns, text)?;
                let short = last_segment(&path);
                if !items.iter().any(|i| i.name() == short) {
                    return Err(EditError::Diagnostics(vec![Diagnostic::error(
                        Default::default(),
                        format!("the text must define `{short}`; rename it with a rename, not by editing the text"),
                    )]));
                }
                let target = self.store.resolve(&path)?.ok_or_else(|| StoreError::NoSuchName(path.clone()))?;
                let mut group = HashSet::new();
                if let Target::Content(d) = target.target {
                    if let Some(r) = self.store.record(&d)? {
                        for m in self.store.read(|tx| tx.component(&r.component))? {
                            for n in self.store.read(|tx| tx.names_of(&m.id))? {
                                group.insert(n.path);
                            }
                        }
                    }
                }
                self.apply(&ns, items, warnings, Some((&path, expected)), &group)
            }
            Key::Content(_) => Err(EditError::Refused("a definition is immutable: open it through a name to edit it".into())),
            _ => Err(EditError::Refused(format!("{key:?} cannot be written"))),
        }
    }

    /// Creates definitions from text under a namespace. `name` is Moonkale's
    /// relative path of the new node (`a/b` puts the text in sub-namespace `a`).
    /// Returns the key of `name` if the text defines it.
    pub fn create_text(&self, parent: &Key, name: &str, text: &str) -> R<(Key, Written)> {
        let Key::Derived(s) = parent else { return Err(EditError::Refused("not a namespace".into())) };
        let Some(base) = s.strip_prefix("ns:") else { return Err(EditError::Refused("not a namespace".into())) };
        let rel = path_from_moonkale(name);
        let full = abs(base, &rel);
        let ns = parent_ns(&full).to_string();
        let (items, warnings) = self.parse(&ns, text)?;
        if items.is_empty() {
            return Err(EditError::Diagnostics(vec![Diagnostic::error(Default::default(), "the text defines nothing")]));
        }
        let w = self.apply(&ns, items, warnings, None, &HashSet::new())?;
        let key = if w.targets.iter().any(|(p, _)| *p == full) { Key::name(&full) } else { Key::name(&w.targets[0].0) };
        Ok((key, w))
    }

    /// Creates an empty namespace.
    pub fn create_dir(&self, parent: &Key, name: &str) -> R<Key> {
        let Key::Derived(s) = parent else { return Err(EditError::Refused("not a namespace".into())) };
        let Some(base) = s.strip_prefix("ns:") else { return Err(EditError::Refused("not a namespace".into())) };
        let full = abs(base, &path_from_moonkale(name));
        sophia_store::validate_path(&full)?;
        self.store.set_settings(&format!("ns/{full}"), &[(format!("ns/{full}"), Some(String::new()))])?;
        Ok(Key::ns(&full))
    }

    /// Moves a name to `to`, Moonkale's path relative to the source root.
    pub fn rename(&self, key: &Key, to: &str) -> R<Key> {
        let Key::Derived(s) = key else { return Err(EditError::Refused("only names can be renamed".into())) };
        let Some(from) = s.strip_prefix("name:") else { return Err(EditError::Refused("only names can be renamed".into())) };
        let to = path_from_moonkale(to);
        self.store.rename(from, &to, None)?;
        Ok(Key::name(&to))
    }

    /// Removes a name; the definition stays.
    pub fn delete(&self, key: &Key) -> R<()> {
        let Key::Derived(s) = key else { return Err(EditError::Refused("only names can be deleted".into())) };
        if let Some(p) = s.strip_prefix("name:") {
            self.store.remove(p, None)?;
            return Ok(());
        }
        if let Some(p) = s.strip_prefix("ns:") {
            if self.store.read(|tx| tx.names_under(p, 1))?.is_empty() {
                self.store.set_settings(&format!("ns/{p}"), &[(format!("ns/{p}"), None)])?;
                return Ok(());
            }
            return Err(EditError::Refused("a namespace with names cannot be deleted".into()));
        }
        Err(EditError::Refused(format!("{key:?} cannot be deleted")))
    }

    fn write_settings(&self, expected: u64, text: &str) -> R<Written> {
        let found = self.settings_version()?;
        if found != expected {
            return Err(EditError::Conflict { expected, found });
        }
        let rows = sophialang_syntax::profile::parse_settings(text)
            .map_err(|m| EditError::Diagnostics(vec![Diagnostic::error(Default::default(), m)]))?;
        let mine = self.personal_profile();
        let old = profiles::own_settings(&self.store, &mine)?;
        let mut changes: Vec<(String, Option<String>)> =
            old.keys().filter(|k| !rows.contains_key(*k)).map(|k| (format!("profile/{mine}/{k}"), None)).collect();
        changes.extend(rows.iter().map(|(k, v)| (format!("profile/{mine}/{k}"), Some(v.clone()))));
        changes.push((format!("reader/{}/default", self.reader), Some(mine.clone())));
        let version = found + 1;
        changes.push((format!("settings-version/{}", self.reader), Some(version.to_string())));
        self.store.set_settings(&format!("reader/{}", self.reader), &changes)?;
        Ok(Written { version, targets: Vec::new(), warnings: Vec::new(), stale_dependants: Vec::new() })
    }

    /// Loads the sample program (SPEC-004 §9) under `sample.*`, with its
    /// relations as nominal entities and their members attached.
    pub fn load_sample(&self) -> Result<(), StoreError> {
        use sophialang_core::samples;
        let (names, defs): (Vec<_>, Vec<Definition>) = samples::program().into_iter().unzip();
        let records = sophialang_core::normalise_batch(&defs, &mut Default::default())
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let new: Vec<(String, usize, Visibility)> = names
            .iter()
            .enumerate()
            .filter(|(_, n)| self.store.resolve(n).ok().flatten().is_none())
            .map(|(i, n)| (n.to_string(), i, Visibility::Public))
            .collect();
        self.store.add(&records, &new)?;
        for (path, kind, id) in samples::nominals() {
            self.store.ensure_nominal(id, sophialang_core::LANG, kind.as_str(), path)?;
            if self.store.resolve(path)?.is_none() {
                self.store.alias(path, Target::Nominal(id), Visibility::Public)?;
            }
        }
        for (rel, i) in samples::memberships() {
            self.store.add_method(&rel, &records[i].id)?;
        }
        for b in sophialang_syntax::resolve::BUILTINS {
            let kind = if b.chars().next().is_some_and(char::is_uppercase) { "primitive" } else { "generic" };
            self.store.ensure_nominal(samples::builtin(b), sophialang_core::LANG, kind, b)?;
        }
        Ok(())
    }
}
