//! Part 3 of the Julia conformance suite (dispatch), on the fixture
//! `julia-semantics/dispatch`: what Sophia says each module-level name in a
//! definition refers to, against Julia's lowered code (`dispatch/truth.tsv`,
//! printed by `julia-semantics/calls.jl`; see `calls.md` to regenerate).
//! Each case's expected outcome is the ledger entry of [[Julia Conformance]].

use std::path::Path;

use sophia_ingest::conformance::{Outcome, dispatch_ledger};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/julia-semantics/dispatch");

#[test]
fn dispatch_ledger_on_the_fixture() {
    let truth = std::fs::read_to_string(Path::new(DIR).join("truth.tsv")).unwrap();
    let ledger = dispatch_ledger(Path::new(DIR), &truth).unwrap();
    let outcome = |caller: &str, name: &str| {
        let r: Vec<_> = ledger.rows.iter().filter(|r| r.caller == caller && r.name == name).collect();
        assert_eq!(r.len(), 1, "{caller} → {name}: {r:?}");
        r[0].outcome
    };
    // One method, nothing adds another: the content reference is right.
    assert_eq!(outcome("call_solo", "solo"), Outcome::KnowsOnlyDefinition);
    // Several methods: the function, its method left open.
    assert_eq!(outcome("call_duo", "duo"), Outcome::KnowsFunction);
    assert_eq!(outcome("call_in_closure", "duo"), Outcome::KnowsFunction);
    assert_eq!(outcome("call_in_generator", "duo"), Outcome::KnowsFunction);
    // WRONG (reported to Moonkale): a second method added by a qualified
    // head (`Inner.h(x::Int)`) or after `import ..Disp: lone` in another
    // file; the crate sees one definition and refers to it by content, so
    // the id says the call reaches that method. Julia's `call_h(1)` reaches
    // the other one.
    assert_eq!(outcome("call_h", "h"), Outcome::WrongOneOfMany);
    assert_eq!(outcome("call_lone", "lone"), Outcome::WrongOneOfMany);
    // Default arguments: Julia's two methods are both that definition.
    assert_eq!(outcome("call_defarg", "defarg"), Outcome::KnowsOnlyDefinition);
    // WRONG (reported to Moonkale): `$duo` in an `@eval` template is the
    // loop's variable, but Sophia records a use of the function `duo`.
    assert_eq!(outcome("tagged", "duo"), Outcome::WrongInterpolated);
    // A multi-line definition.
    assert_eq!(outcome("multi", "solo"), Outcome::KnowsOnlyDefinition);
    assert_eq!(outcome("multi", "duo"), Outcome::KnowsFunction);
    // Two definitions on one line cannot be told apart by Julia's line: no row
    // for either (review V1).
    assert!(!ledger.rows.iter().any(|r| r.caller == "same_a" || r.caller == "same_b"));
    // One name under two bindings: matched by binding, not by name (review V2).
    let both: Vec<_> = ledger.rows.iter().filter(|r| r.caller == "both" && r.name == "h").map(|r| r.outcome).collect();
    assert_eq!(both, vec![Outcome::KnowsOnlyDefinition, Outcome::OpenHere], "{both:?}");
    // A type: the binding.
    assert_eq!(outcome("make", "Point"), Outcome::KnowsBinding);
    // A function passed as a value is still that one method.
    assert_eq!(outcome("pass_solo", "solo"), Outcome::KnowsOnlyDefinition);
    // Left open: an imported name, a qualified name, Base's functions.
    assert_eq!(outcome("call_length", "length"), Outcome::OpenHere); // Base.length, with a method here
    assert_eq!(outcome("call_qualified", "h"), Outcome::OpenHere);
    assert_eq!(outcome("call_sum", "sum"), Outcome::OpenOutside);
    // A parameter named like a function: no claim on either side.
    for (caller, name) in [("shadow", "duo"), ("shadow_solo", "solo")] {
        assert!(!ledger.rows.iter().any(|r| r.caller == caller && r.name == name), "{caller} → {name}");
    }
    // Nothing else is wrong.
    let wrong: Vec<_> = ledger
        .rows
        .iter()
        .filter(|r| matches!(r.outcome, Outcome::WrongOneOfMany | Outcome::WrongTarget | Outcome::WrongInterpolated))
        .collect();
    assert_eq!(wrong.len(), 3, "{wrong:#?}");
}
