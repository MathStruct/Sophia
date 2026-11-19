//! Proof trust records (SPEC-006 §5, E-0, B-021): what an external checker
//! said about each proof, stored as annotations, and the query that asks
//! which theorems rest only on checked proofs. See `trust.md`.

use std::collections::{BTreeMap, BTreeSet};

use sophia_hash::Digest;
use sophia_hash::canonical::Writer;
use sophia_store::Annotation;

use crate::metamath::Assertion;

/// The annotation kind of a proof the checker accepted (status `checked`).
pub const CHECKED: &str = "checked";
/// The annotation kind of a proof the checker rejected (status `failed`).
pub const CHECK_FAILED: &str = "check-failed";

/// A checker, as SPEC-006 §5 names it: the program and its version (the
/// annotation's `checker` and `checker_version`), and the trusted kernel it
/// reduces to (in the payload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checker {
    pub name: String,
    pub version: String,
    pub kernel: String,
}

/// The payload of a check annotation: the kernel, in canonical encoding.
/// Certificates and assumptions are not recorded: a Metamath proof is its
/// own certificate (it is in the store), and its assumptions are the axioms
/// and definitions its references reach, which a query finds.
fn payload(checker: &Checker) -> Vec<u8> {
    let mut w = Writer::new();
    w.str("sophia", "proof-check-v0");
    w.str("kernel", &checker.kernel);
    w.finish()
}

/// One annotation per distinct proof: [`CHECKED`] unless one of the labels
/// whose proof it is is in `failed`, then [`CHECK_FAILED`]. `inputs` are the
/// ids the check ran against (the library's source manifest): a different
/// input gives the same annotation id with other inputs, as a re-check.
/// Labels in `failed` with no proof are returned apart, as `unknown`.
pub fn proof_checks(
    checker: &Checker,
    assertions: &[Assertion],
    failed: &BTreeSet<String>,
    inputs: &[Digest],
) -> (Vec<Annotation>, Vec<String>) {
    // Two labels with one proof (a repeated theorem) are one record: it
    // fails if either label failed.
    let mut verdict: BTreeMap<Digest, bool> = BTreeMap::new();
    let mut known = BTreeSet::new();
    for a in assertions {
        if let Some(p) = &a.proof {
            known.insert(a.label.as_str());
            let ok = verdict.entry(p.id).or_insert(true);
            *ok &= !failed.contains(&a.label);
        }
    }
    let unknown = failed.iter().filter(|l| !known.contains(l.as_str())).cloned().collect();
    let body = payload(checker);
    let anns = verdict
        .into_iter()
        .map(|(proof, ok)| {
            let kind = if ok { CHECKED } else { CHECK_FAILED };
            let mut a = Annotation::new(kind, vec![proof], body.clone(), Some((&checker.name, &checker.version)));
            a.status = if ok { "checked" } else { "failed" }.into();
            a.inputs = inputs.to_vec();
            a
        })
        .collect();
    (anns, unknown)
}

/// The labels `metamath.exe`'s `verify proof *` reports an error for. Each
/// error starts with a line naming the statement:
/// `?Error on line 12 of file "x.mm" at statement 9, label "th1", type "$p":`
/// (the label may also appear on a following line when the message wraps,
/// so `set width` should be large).
pub fn metamath_verify_failures(output: &str) -> BTreeSet<String> {
    let mut failed = BTreeSet::new();
    for line in output.lines() {
        if !line.trim_start().starts_with("?Error") {
            continue;
        }
        if let Some(rest) = line.split("label \"").nth(1)
            && let Some(label) = rest.split('"').next()
        {
            failed.insert(label.to_string());
        }
    }
    failed
}

/// The proofs that rest only on checked proofs: a proof is *tainted* if it
/// has no [`CHECKED`] annotation, or references a tainted proof; the rest are
/// returned. Linear in the references (taint flows up once), not the
/// transitive closure per theorem. One column: the proof id.
pub const RESTS_ON_CHECKED: &str = "query rests_on_checked()
 tainted(p) = def(p, :proof) && !checked(p)
 tainted(p) = refs(p, q) && tainted(q) && def(p, :proof)
 checked(p) = annotation(_, \"checked\", p)
 select p where def(p, :proof) && !tainted(p)
end";

/// The proofs that do not: the complement of [`RESTS_ON_CHECKED`].
pub const RESTS_ON_UNCHECKED: &str = "query rests_on_unchecked()
 tainted(p) = def(p, :proof) && !checked(p)
 tainted(p) = refs(p, q) && tainted(q) && def(p, :proof)
 checked(p) = annotation(_, \"checked\", p)
 select p where tainted(p)
end";
