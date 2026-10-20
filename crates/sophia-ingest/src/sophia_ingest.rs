//! Frontends that bring real code into Sophia ([[Real Code Corpora]], B-16).
//! Metamath first. See `sophia_ingest.md`.

pub mod metamath;

use std::collections::BTreeMap;

use sophia_store::{Annotation, Backend, Store, StoreError, Visibility};

/// What an ingest did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Assertions read (one name each).
    pub assertions: usize,
    /// Proofs read (one name each, `mmproof.<db>.<label>`).
    pub proofs: usize,
    /// Records that were new: fewer than `assertions` when content repeats.
    pub new_records: usize,
    /// Assertions per role (axiom, definition, theorem, syntax).
    pub kinds: BTreeMap<String, usize>,
}

/// The names of a library's collections (SPEC-006 §1, versioned nominal
/// collections): `mmlib.<db>.proofs`, `.axioms`, `.definitions`.
pub fn library_collection(db: &str, which: &str) -> String {
    format!("mmlib.{db}.{which}")
}

/// Stores a parsed Metamath database, in batches of `batch`:
/// - one record per distinct statement, named `mm.<db>.<label>`;
/// - one record per distinct proof, named `mmproof.<db>.<label>`;
/// - one `proof` annotation per proof, on the statement and the proof, so
///   queries can tell "proves" (the annotation) from "uses" (references);
/// - the library itself as three nominal collections: the proofs it uses,
///   the statements it assumes as axioms, and its definitions. Being an
///   axiom, or the chosen proof, is a library's choice, not a property of a
///   statement: a statement can be an axiom in one library and proved in
///   another, or even in another part of the same library.
pub fn store_metamath<B: Backend>(
    store: &Store<B>,
    db: &str,
    assertions: &[metamath::Assertion],
    batch: usize,
) -> Result<Report, StoreError> {
    let mut report = Report { assertions: assertions.len(), ..Report::default() };
    for a in assertions {
        *report.kinds.entry(a.role.clone()).or_default() += 1;
    }
    let collection = |which: &str| -> Result<sophia_hash::Digest, StoreError> {
        let path = library_collection(db, which);
        match store.resolve(&path)? {
            Some(n) => Ok(n.target.digest()),
            None => store.new_nominal(metamath::LANG, "relation", &path),
        }
    };
    let (proofs_n, axioms_n, defs_n) = (collection("proofs")?, collection("axioms")?, collection("definitions")?);
    for chunk in assertions.chunks(batch.max(1)) {
        let mut records = Vec::new();
        let mut names: Vec<(String, usize, Visibility)> = Vec::new();
        for a in chunk {
            names.push((metamath::name_for(db, &a.label), records.len(), Visibility::Public));
            records.push(a.record.clone());
            if let Some(p) = &a.proof {
                names.push((metamath::proof_name_for(db, &a.label), records.len(), Visibility::Public));
                records.push(p.clone());
                report.proofs += 1;
            }
        }
        report.new_records += store.add(&records, &names)?.new_records;
        let anns: Vec<Annotation> = chunk
            .iter()
            .filter_map(|a| a.proof.as_ref().map(|p| Annotation::new("proof", vec![a.record.id, p.id], Vec::new(), None)))
            .collect();
        if !anns.is_empty() {
            store.annotate_many(&anns)?;
        }
        let pick = |role: &str| -> Vec<sophia_hash::Digest> { chunk.iter().filter(|a| a.role == role).map(|a| a.record.id).collect() };
        store.add_members(&proofs_n, &chunk.iter().filter_map(|a| a.proof.as_ref().map(|p| p.id)).collect::<Vec<_>>())?;
        store.add_members(&axioms_n, &pick("axiom"))?;
        store.add_members(&defs_n, &pick("definition"))?;
    }
    Ok(report)
}
