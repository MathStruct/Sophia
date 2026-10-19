//! Normalising a batch of drafts into hashed records (SPEC-001 §4,
//! SPEC-003 §3). See `normalise.md`.
//!
//! A batch is a list of definitions whose terms may refer to each other with
//! `Term::Draft(i)`. References to definitions already in the store are
//! `Term::Ref`. The batch is split into recursive groups (dependency-first),
//! each group is ordered canonically, and each member gets its content id.

use sophia_hash::component::{self, Budget, OrderError};
use sophia_hash::merkle::{component_digest, member_digest};
use sophia_hash::{Digest, NominalId, Record, Status};

use crate::definition::Definition;
use crate::encode::{Resolved, display_of, encode};
use crate::term::Term;

/// The language tag of sophialang in the store.
pub const LANG: &str = "sophialang";
/// The normaliser's name and version: part of every id (SPEC-001 §4.5).
pub const NORMALISER: &str = "sophialang-core@0";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum NormaliseError {
    #[error("definition {from} refers to draft {to}, which is not in the batch")]
    DanglingDraft { from: usize, to: u32 },
    #[error(transparent)]
    Order(#[from] OrderError),
}

fn drafts_in_order(def: &Definition) -> Vec<u32> {
    let mut v = Vec::new();
    def.walk(&mut |t| {
        if let Term::Draft(i) = t {
            v.push(*i)
        }
    });
    v
}

/// Hashes a batch. Returns one record per input definition, in input order.
pub fn normalise_batch(defs: &[Definition], budget: &mut Budget) -> Result<Vec<Record>, NormaliseError> {
    let n = defs.len();
    let drafts: Vec<Vec<u32>> = defs.iter().map(drafts_in_order).collect();
    for (from, ds) in drafts.iter().enumerate() {
        if let Some(&to) = ds.iter().find(|&&d| d as usize >= n) {
            return Err(NormaliseError::DanglingDraft { from, to });
        }
    }

    let comps = component::strongly_connected(n, |v| drafts[v].iter().map(|&d| d as usize).collect::<Vec<_>>());
    let mut resolved: Vec<Option<Digest>> = vec![None; n];
    let mut out: Vec<Option<Record>> = vec![None; n];

    for comp in comps {
        // Position of each batch index within this component.
        let pos = |i: usize| comp.iter().position(|&m| m == i);
        let refs = |m: usize| drafts[comp[m]].iter().filter_map(|&d| pos(d as usize)).collect::<Vec<_>>();
        let enc = |m: usize, index: &[u32]| {
            encode(&defs[comp[m]], &|d| match pos(d as usize) {
                Some(p) => Resolved::Member(index[p]),
                None => Resolved::Ref(resolved[d as usize].expect("dependency-first order")),
            })
        };
        let ordered = component::canonical_order(comp.len(), refs, enc, budget)?;
        let cdigest = component_digest(LANG, NORMALISER, &ordered.serialisation);

        for (p, &i) in comp.iter().enumerate() {
            let id = member_digest(&cdigest, ordered.index[p]);
            resolved[i] = Some(id);
        }
        for (p, &i) in comp.iter().enumerate() {
            let def = &defs[i];
            let mut refs: Vec<Digest> = Vec::new();
            let mut nominal_refs: Vec<NominalId> = Vec::new();
            def.walk(&mut |t| match t {
                Term::Ref(d) => refs.push(*d),
                Term::Draft(d) if pos(*d as usize).is_none() => refs.push(resolved[*d as usize].unwrap()),
                Term::Nominal(id) => nominal_refs.push(*id),
                _ => {}
            });
            refs.sort();
            refs.dedup();
            nominal_refs.sort();
            nominal_refs.dedup();
            out[i] = Some(Record {
                id: resolved[i].unwrap(),
                lang: LANG.into(),
                normaliser: NORMALISER.into(),
                kind: def.kind.as_str().into(),
                component: cdigest,
                index: ordered.index[p],
                component_size: comp.len() as u32,
                bytes: ordered.bytes[p].clone(),
                display: display_of(def),
                status: if def.is_incomplete() { Status::Incomplete } else { Status::Complete },
                refs,
                nominal_refs,
            });
        }
    }
    Ok(out.into_iter().map(|r| r.expect("every definition is in a component")).collect())
}

/// Hashes one definition with no drafts.
pub fn normalise_one(def: &Definition) -> Result<Record, NormaliseError> {
    Ok(normalise_batch(std::slice::from_ref(def), &mut Budget::default())?.remove(0))
}
