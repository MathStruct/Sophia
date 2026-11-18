//! Outside edits of a generated sophialang file as proposals (SPEC-010 §5):
//! `propose_text` parses the edited text and reports what it would change,
//! without writing. See `propose.md`.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use sophia_hash::Digest;
use sophia_store::{Backend, FileProposal, FileReport, Proposal, StoreError, Target};
use sophialang_syntax::{Diagnostic, Item, Limits, Severity};

use crate::edit::{Normalised, abs, head_relation, normalise_items};
use crate::{ExportedFile, Interface, Result, StoreEnv};

/// What `propose_text` found; nothing was written.
#[derive(Debug, Default)]
pub struct TextProposal {
    /// One `FileProposal` whose `path` is the namespace, unless the text
    /// has errors. `conflicts` holds (namespace, given token, current
    /// export token) when the namespace changed since it was exported.
    /// `report` counts what the file says; `error` is a refusal an edit
    /// would raise (`LaidOut`, `NameClaimed`, `NameExists`, `InvalidPath`).
    pub proposal: Proposal,
    /// Parse and normalisation errors (then `proposal.files` is empty),
    /// and the parser's warnings. Ranges are into the text.
    pub diagnostics: Vec<Diagnostic>,
}

impl TextProposal {
    /// The text says what the store holds: no conflict, no error, and no
    /// name or membership would change (SPEC-010 acceptance 3).
    pub fn changes_nothing(&self) -> bool {
        let p = &self.proposal;
        p.conflicts.is_empty()
            && p.error.is_none()
            && !self.diagnostics.iter().any(|d| d.severity == Severity::Error)
            && p.generics_added.is_empty()
            && p.generics_removed.is_empty()
            && p.files.iter().all(|f| {
                f.names_added.is_empty()
                    && f.names_removed.is_empty()
                    && f.names_retargeted.is_empty()
                    && f.members_added.is_empty()
                    && f.members_removed.is_empty()
            })
    }
}

/// Why a text cannot be applied: a store refusal, or text the normaliser
/// rejects.
enum Refusal {
    Store(StoreError),
    Text(Diagnostic),
}

impl From<StoreError> for Refusal {
    fn from(e: StoreError) -> Self {
        Refusal::Store(e)
    }
}

/// A stand-in id for a relation the text introduces. The real one is
/// random (`Store::new_nominal`), so records that refer to a new relation
/// get other ids when the edit is made.
fn provisional_relation(path: &str) -> Digest {
    let mut h = blake3::Hasher::new();
    h.update(b"sophia-proposal-relation-v1");
    h.update(path.as_bytes());
    sophia_hash::merkle::nominal_id(sophialang_core::LANG, "relation", h.finalize().as_bytes())
}

impl<B: Backend + Sync> Interface<B> {
    /// What saving `text` as the generated file of `namespace` would change
    /// (SPEC-010 §5): the text is parsed with Sophia's parser in the
    /// namespace and compared with the namespace's current export. `token`
    /// is the export token Moonkale kept as the file's marker; a different
    /// current token is a conflict. Writes nothing and takes no lock.
    pub fn propose_text(&self, namespace: &str, text: &str, token: u64) -> Result<TextProposal> {
        if self.store.is_derived()? {
            return Err(StoreError::Derived);
        }
        let (file, old) = self.export_with_targets(namespace)?;
        let mut out = TextProposal::default();
        if file.token != token {
            out.proposal.conflicts.push((namespace.to_string(), token, file.token));
        }
        let env = StoreEnv { store: &self.store };
        let items = match sophialang_syntax::parse_text(&env, namespace, text, Limits::default()) {
            Ok((items, warnings)) => {
                out.diagnostics = warnings;
                items
            }
            Err(errors) => {
                out.diagnostics = errors;
                return Ok(out);
            }
        };
        let aliases: HashSet<&str> = file.aliases.iter().map(|(a, _)| a.as_str()).collect();
        let mut fp = FileProposal { path: namespace.to_string(), text_changed: text != file.text, ..FileProposal::default() };
        match self.diff_items(namespace, &items, &old, &aliases, &mut fp) {
            Ok(report) => {
                out.proposal.report = Some(report);
                out.proposal.files.push(fp);
            }
            Err(Refusal::Store(StoreError::Backend(e))) => return Err(StoreError::Backend(e)),
            Err(Refusal::Store(e)) => {
                out.proposal.error = Some(e);
                out.proposal.files.push(fp);
            }
            Err(Refusal::Text(d)) => out.diagnostics.push(d),
        }
        Ok(out)
    }

    /// The namespace's export and the target of each name it covers
    /// (printed names and aliases), read at one log position.
    fn export_with_targets(&self, namespace: &str) -> Result<(ExportedFile, BTreeMap<String, Target>)> {
        for _ in 0..8 {
            let seq = self.store.last_seq()?;
            let file = self.export(namespace)?;
            let (targets, now) = self.store.read(|tx| {
                let mut m = BTreeMap::new();
                for n in file.spans.iter().map(|(n, _, _)| n).chain(file.aliases.iter().map(|(a, _)| a)) {
                    if let Some(e) = tx.name(n)? {
                        m.insert(n.clone(), e.target);
                    }
                }
                Ok((m, tx.last_seq()?))
            })?;
            if now == seq {
                return Ok((file, targets));
            }
        }
        Err(StoreError::Backend(format!("proposal for {namespace:?}: the namespace kept changing")))
    }

    fn laid_out_refusal(&self, name: &str) -> std::result::Result<(), Refusal> {
        match self.store.laid_out(name)? {
            Some((path, token)) => Err(Refusal::Store(StoreError::LaidOut { path, token })),
            None => Ok(()),
        }
    }

    /// Fills `fp` with the names and memberships the items change against
    /// `old` (the export's names), refusing as `apply` would.
    fn diff_items(
        &self,
        ns: &str,
        items: &[Item],
        old: &BTreeMap<String, Target>,
        aliases: &HashSet<&str>,
        fp: &mut FileProposal,
    ) -> std::result::Result<FileReport, Refusal> {
        // The refusals of `apply`, with the file's names as the group that
        // may be updated.
        for it in items {
            let p = abs(ns, it.name());
            sophia_store::validate_path(&p)?;
            if let Some(rel) = head_relation(it.definition()) {
                for n in self.store.read(|tx| tx.names_of(&rel))? {
                    self.laid_out_refusal(&n.path)?;
                }
            }
            if let Some(existing) = self.store.resolve(&p)? {
                self.laid_out_refusal(&p)?;
                match (it, existing.target) {
                    (Item::Relation { .. }, Target::Content(_)) => return Err(StoreError::NameExists(p).into()),
                    (Item::Relation { .. }, Target::Nominal(_)) => {}
                    _ if old.contains_key(&p) => {}
                    _ => return Err(StoreError::NameClaimed { name: p, owner: String::new() }.into()),
                }
            }
        }

        let mut nominal_of: BTreeMap<usize, Digest> = BTreeMap::new();
        let mut provisional: BTreeSet<Digest> = BTreeSet::new();
        for (i, it) in items.iter().enumerate() {
            if let Item::Relation { name, .. } = it {
                let p = abs(ns, name);
                let id = match self.store.resolve(&p)?.map(|n| n.target) {
                    Some(Target::Nominal(id)) => id,
                    _ => {
                        let id = provisional_relation(&p);
                        provisional.insert(id);
                        id
                    }
                };
                nominal_of.insert(i, id);
            }
        }
        let Normalised { def_index, defs, sigs, n_defs, records } =
            normalise_items(items, &nominal_of).map_err(|e| Refusal::Text(Diagnostic::error(Default::default(), e.to_string())))?;

        // Names.
        let mut defined: BTreeSet<String> = BTreeSet::new();
        for (k, &i) in def_index.iter().enumerate() {
            let p = abs(ns, items[i].name());
            let id = records[k].id;
            match self.store.resolve(&p)?.map(|n| n.target.digest()) {
                None => fp.names_added.push((p.clone(), id)),
                Some(o) if o != id => fp.names_retargeted.push((p.clone(), o, id)),
                Some(_) => {}
            }
            defined.insert(p);
        }
        for (&i, id) in &nominal_of {
            let p = abs(ns, items[i].name());
            if provisional.contains(id) {
                fp.names_added.push((p.clone(), *id));
            }
            defined.insert(p);
        }
        for (p, t) in old {
            if !defined.contains(p) && !aliases.contains(p.as_str()) {
                fp.names_removed.push((p.clone(), t.digest()));
            }
        }

        // Memberships: a signature replaces its relation's old one; a rule
        // joins its head's relation and replaces its own old version.
        let relation_name = |rel: &Digest| -> std::result::Result<String, Refusal> {
            if let Some((&i, _)) = nominal_of.iter().find(|(_, id)| *id == rel) {
                return Ok(abs(ns, items[i].name()));
            }
            let names = self.store.read(|tx| tx.names_of(rel))?;
            Ok(names.into_iter().map(|n| n.path).min().unwrap_or_else(|| rel.to_hex()))
        };
        let methods_of = |rel: &Digest| -> std::result::Result<Vec<Digest>, Refusal> {
            if provisional.contains(rel) { Ok(Vec::new()) } else { Ok(self.store.methods(rel)?) }
        };
        for (s, (i, _)) in sigs.iter().enumerate() {
            let rel = nominal_of[i];
            let sig = records[n_defs + s].id;
            let name = abs(ns, items[*i].name());
            let methods = methods_of(&rel)?;
            if !methods.contains(&sig) {
                fp.members_added.push((name.clone(), sig));
            }
            for m in methods {
                if m != sig && self.store.record(&m)?.is_some_and(|r| r.kind == "relation") {
                    fp.members_removed.push((name.clone(), m));
                }
            }
        }
        for (k, def) in defs.iter().take(n_defs).enumerate() {
            let Some(rel) = head_relation(def) else { continue };
            if !provisional.contains(&rel) && self.store.nominal(&rel)?.is_none() {
                continue;
            }
            let id = records[k].id;
            let name = relation_name(&rel)?;
            let methods = methods_of(&rel)?;
            if !methods.contains(&id) {
                fp.members_added.push((name.clone(), id));
            }
            let p = abs(ns, items[def_index[k]].name());
            if let Some((_, o, _)) = fp.names_retargeted.iter().find(|(q, _, _)| *q == p) {
                if methods.contains(o) {
                    fp.members_removed.push((name, *o));
                }
            }
        }
        fp.members_added.sort();
        fp.members_added.dedup();
        fp.members_removed.sort();
        fp.members_removed.dedup();

        let mut new_records = 0;
        for r in &records {
            if self.store.record(&r.id)?.is_none() {
                new_records += 1;
            }
        }
        Ok(FileReport {
            files: 1,
            new_records,
            names_added: fp.names_added.len(),
            names_moved: fp.names_retargeted.len(),
            names_removed: fp.names_removed.len(),
            members_added: fp.members_added.len(),
            members_removed: fp.members_removed.len(),
            ..FileReport::default()
        })
    }
}
