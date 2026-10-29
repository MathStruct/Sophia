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

/// Text that starts with `query` or `select` is a query (SPEC-007 §8);
/// neither is a sophialang keyword.
fn looks_like_query(text: &str) -> bool {
    let first = text.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#')).unwrap_or("");
    first.starts_with("query ") || first.starts_with("select ")
}

fn parse_query(text: &str) -> R<sophia_query::Prepared> {
    sophia_query::prepare_text(text).map_err(|e| match e {
        sophia_query::Error::Parse { at, message } => {
            let end = text[at.min(text.len())..].chars().next().map(|c| at + c.len_utf8()).unwrap_or(at);
            EditError::Diagnostics(vec![Diagnostic::error(sophialang_syntax::Span::new(at, end), message)])
        }
        e => EditError::Diagnostics(vec![Diagnostic::error(Default::default(), e.to_string())]),
    })
}

fn query_error(e: sophia_query::Error) -> EditError {
    match e {
        sophia_query::Error::Store(s) => EditError::Store(s),
        e => EditError::Refused(e.to_string()),
    }
}

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
            // A rule joins its head's relation: refused when that is a
            // generic function a file lays out (SPEC-010 §2).
            if let Some(rel) = head_relation(it.definition()) {
                for n in self.store.read(|tx| tx.names_of(&rel))? {
                    self.refuse_if_laid_out(&n.path)?;
                }
            }
            let is_edited = edited.is_some_and(|(e, _)| e == p);
            if let Some(existing) = self.store.resolve(&p)? {
                // Every name the text would update, not only the edited one:
                // a group's other names may be laid out (SPEC-010 §2).
                self.refuse_if_laid_out(&p)?;
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
    /// In a folder store (SPEC-009 §3) the files are the truth: every edit
    /// through the interface is refused; the producer writes with
    /// `Store::replace_files`.
    fn refuse_if_derived(&self) -> R<()> {
        if self.store.is_derived()? {
            return Err(EditError::Refused("this is a folder store: the files are the truth, edit them instead".into()));
        }
        Ok(())
    }

    /// In a store-truth folder (SPEC-010 §2) a name that a file lays out is
    /// edited through the file: Moonkale splices the edit into the file's
    /// text and writes it with `replace_files_checked`. The error names the
    /// file and its current token.
    fn refuse_if_laid_out(&self, name: &str) -> R<()> {
        match self.store.laid_out(name)? {
            Some((path, token)) => Err(EditError::Store(StoreError::LaidOut { path, token })),
            None => Ok(()),
        }
    }

    pub fn write_text(&self, key: &Key, expected: u64, text: &str) -> R<Written> {
        self.refuse_if_derived()?;
        match key {
            Key::Derived(s) if s == "settings" => self.write_settings(expected, text),
            Key::Derived(s) if s.starts_with("name:") => {
                let path = s[5..].to_string();
                self.refuse_if_laid_out(&path)?;
                let ns = parent_ns(&path).to_string();
                if self.is_query(&path)? {
                    return self.write_query(&path, expected, text);
                }
                let (items, warnings) = self.parse(&ns, text)?;
                let short = last_segment(&path);
                if !items.iter().any(|i| i.name() == short) {
                    return Err(EditError::Diagnostics(vec![Diagnostic::error(
                        Default::default(),
                        format!("the text must define `{short}`; rename it with a rename, not by editing the text"),
                    )]));
                }
                let found = self.text_token(&path)?;
                if found != expected {
                    return Err(EditError::Conflict { expected, found });
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
                let mut w = self.apply(&ns, items, warnings, Some((&path, target.seq)), &group)?;
                w.version = self.text_token(&path)?;
                Ok(w)
            }
            Key::Content(_) => Err(EditError::Refused("a definition is immutable: open it through a name to edit it".into())),
            _ => Err(EditError::Refused(format!("{key:?} cannot be written"))),
        }
    }

    fn is_query(&self, path: &str) -> R<bool> {
        Ok(match self.store.resolve(path)?.map(|n| n.target) {
            Some(Target::Content(d)) => self.store.record(&d)?.is_some_and(|r| r.lang == sophia_query::canon::LANG),
            _ => false,
        })
    }

    /// Replaces a stored query by edited text: the name moves to the new
    /// query's id, like any edit.
    fn write_query(&self, path: &str, expected: u64, text: &str) -> R<Written> {
        let p = parse_query(text)?;
        let found = self.text_token(path)?;
        if found != expected {
            return Err(EditError::Conflict { expected, found });
        }
        let seq = self.store.resolve(path)?.ok_or_else(|| StoreError::NoSuchName(path.into()))?.seq;
        let record = self.store.read(|tx| p.record(tx).map_err(|e| StoreError::Backend(e.to_string())))?;
        let report = self.store.update(path, &[record], 0, seq)?;
        Ok(Written {
            version: self.text_token(path)?,
            targets: vec![(path.to_string(), p.id)],
            warnings: Vec::new(),
            stale_dependants: report.dependants_of_old,
        })
    }

    /// Creates definitions from text under a namespace. `name` is Moonkale's
    /// relative path of the new node (`a/b` puts the text in sub-namespace `a`).
    /// Returns the key of `name` if the text defines it.
    pub fn create_text(&self, parent: &Key, name: &str, text: &str) -> R<(Key, Written)> {
        self.refuse_if_derived()?;
        let Key::Derived(s) = parent else { return Err(EditError::Refused("not a namespace".into())) };
        let Some(base) = s.strip_prefix("ns:") else { return Err(EditError::Refused("not a namespace".into())) };
        let rel = path_from_moonkale(name);
        let full = abs(base, &rel);
        let ns = parent_ns(&full).to_string();
        if looks_like_query(text) {
            let p = parse_query(text)?;
            sophia_query::save(&self.store, &p, Some(&full)).map_err(query_error)?;
            let version = self.text_token(&full)?;
            return Ok((Key::name(&full), Written { version, targets: vec![(full, p.id)], warnings: Vec::new(), stale_dependants: Vec::new() }));
        }
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
        self.refuse_if_derived()?;
        let Key::Derived(s) = parent else { return Err(EditError::Refused("not a namespace".into())) };
        let Some(base) = s.strip_prefix("ns:") else { return Err(EditError::Refused("not a namespace".into())) };
        let full = abs(base, &path_from_moonkale(name));
        sophia_store::validate_path(&full)?;
        self.store.set_settings(&format!("ns/{full}"), &[(format!("ns/{full}"), Some(String::new()))])?;
        Ok(Key::ns(&full))
    }

    /// Moves a name to `to`, Moonkale's path relative to the source root.
    pub fn rename(&self, key: &Key, to: &str) -> R<Key> {
        self.refuse_if_derived()?;
        let Key::Derived(s) = key else { return Err(EditError::Refused("only names can be renamed".into())) };
        let Some(from) = s.strip_prefix("name:") else { return Err(EditError::Refused("only names can be renamed".into())) };
        self.refuse_if_laid_out(from)?;
        let to = path_from_moonkale(to);
        self.store.rename(from, &to, None)?;
        Ok(Key::name(&to))
    }

    /// Removes a name; the definition stays.
    pub fn delete(&self, key: &Key) -> R<()> {
        self.refuse_if_derived()?;
        let Key::Derived(s) = key else { return Err(EditError::Refused("only names can be deleted".into())) };
        if let Some(p) = s.strip_prefix("name:") {
            self.refuse_if_laid_out(p)?;
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
