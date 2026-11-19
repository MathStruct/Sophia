//! Part 6 of the Julia conformance suite (closures and captured variables),
//! on the fixture `julia-semantics/closures`: for each closure Julia's
//! lowering made (`closures/truth.tsv`, printed by
//! `julia-semantics/closures.jl`; see `closures.md` to regenerate), each
//! name in it, classified by Julia (captured, the closure's own, global) and
//! by Sophia's encoding. Each case's expected outcome is the ledger entry of
//! [[Julia Conformance]] part 6.

use std::path::Path;

use sophia_ingest::conformance::{ClosureLedger, ClosureOutcome, closure_ledger};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/julia-semantics/closures");

fn truth() -> String {
    std::fs::read_to_string(Path::new(DIR).join("truth.tsv")).unwrap()
}

fn ledger(truth: &str) -> ClosureLedger {
    closure_ledger(Path::new(DIR), truth).unwrap()
}

/// (name, Julia's class, outcome) of the rows at a line.
fn at(l: &ClosureLedger, line: usize) -> Vec<(&str, &str, ClosureOutcome)> {
    l.rows.iter().filter(|r| r.line == line).map(|r| (r.name.as_str(), r.julia.as_str(), r.outcome)).collect()
}

#[test]
fn closure_ledger_on_the_fixture() {
    use ClosureOutcome::*;
    let l = ledger(&truth());
    assert_eq!((l.julia_closures, l.matched), (41, 35));
    assert!(l.outside.is_empty() && l.unmatched.is_empty(), "{l:?}");
    // Left open: definitions Sophia refuses (`global` in a body, an inner
    // named function, short form or not).
    let refused: Vec<usize> = l.refused.iter().map(|r| r.1).collect();
    assert_eq!(refused, vec![30, 39, 40, 81]);
    // Captured: a parameter, one named like a module constant, a boxed
    // variable the closure assigns, a local assigned only after the
    // closure (Julia's local scope is the whole function), a static
    // parameter, a loop variable, a `let` variable.
    assert_eq!(at(&l, 15), vec![("x", "own", KnowsOwn), ("n", "captured", KnowsCaptured)]);
    assert_eq!(at(&l, 19), vec![("x", "own", KnowsOwn), ("k", "captured", KnowsCaptured)]);
    assert_eq!(at(&l, 21), vec![("c", "captured:box", KnowsCaptured)]);
    assert_eq!(at(&l, 23), vec![("z", "captured:box", KnowsCaptured)]);
    assert_eq!(at(&l, 46), vec![("T", "captured:sparam", KnowsCaptured)]);
    assert_eq!(at(&l, 51), vec![("i", "captured", KnowsCaptured)]);
    assert_eq!(at(&l, 69), vec![("b", "captured", KnowsCaptured)]);
    // `late_solo`: a module function `solo` exists, and the closure's
    // `solo` is still the local assigned after it.
    assert_eq!(at(&l, 78), vec![("solo", "captured:box", KnowsCaptured)]);
    // Own: assigned only in the closure (also a module global's name),
    // `local` shadowing, a parameter shadowing an outer local.
    assert_eq!(at(&l, 25), vec![("y", "own", KnowsOwn)]);
    assert_eq!(at(&l, 28), vec![("total", "own", KnowsOwn)]);
    assert_eq!(at(&l, 57), vec![("x", "own", KnowsOwn)]);
    assert_eq!(at(&l, 62), vec![("x", "own", KnowsOwn)]);
    // Global: a module constant, a function, a name only another closure
    // assigns.
    assert_eq!(at(&l, 17), vec![("x", "own", KnowsOwn), ("scale", "global", KnowsGlobal)]);
    assert_eq!(at(&l, 66), vec![("solo", "global", KnowsGlobal), ("a", "captured", KnowsCaptured)]);
    assert_eq!(at(&l, 94), vec![("q", "own", KnowsOwn), ("q", "global", KnowsGlobal)]);
    // Nested closures: the outer one captures what the inner one needs.
    let curry = at(&l, 32);
    assert_eq!(curry.len(), 5);
    assert!(curry.iter().all(|r| r.2 == KnowsCaptured || r.2 == KnowsOwn));
    assert_eq!(curry.iter().filter(|r| r.0 == "a" && r.2 == KnowsCaptured).count(), 2);
    // Generators and comprehensions (Julia files their bodies at no line).
    assert_eq!(at(&l, 72), vec![("x", "own", KnowsOwn), ("a", "captured", KnowsCaptured)]);
    assert_eq!(at(&l, 73), vec![("x", "own", KnowsOwn), ("a", "captured", KnowsCaptured)]);
    // A `do` block: Julia's line is its body's.
    assert_eq!(at(&l, 35), vec![("x", "own", KnowsOwn), ("a", "captured", KnowsCaptured)]);
    // Several closures on one line, paired in source order.
    assert_eq!(at(&l, 104).len(), 5);
    assert_eq!(at(&l, 106).len(), 6);
    // Left open: a keyword parameter's name is part of the encoding, and a
    // keyword argument's name is lost (below), so renaming cannot decide.
    assert_eq!(at(&l, 75), vec![("x", "own", KnowsOwn), ("d", "own", Undecided), ("a", "captured", KnowsCaptured)]);
    assert_eq!(at(&l, 96), vec![("isapprox", "global", KnowsGlobal), ("atol", "captured", Undecided)]);
    // Review B-076 (Mistral), 1: the `do` closure (Julia: its body's line
    // 110) is the `do` block, not the arrow starting on that line; the
    // `do` block's names include the arrow's (it creates the arrow).
    let d: Vec<(&str, &str, &str, ClosureOutcome)> =
        l.rows.iter().filter(|r| r.line == 110).map(|r| (r.closure.as_str(), r.name.as_str(), r.julia.as_str(), r.outcome)).collect();
    assert_eq!(
        d,
        vec![
            ("#do_inner##0#do_inner##1", "x", "own", KnowsOwn),
            ("#do_inner##0#do_inner##1", "a", "captured", KnowsCaptured),
            ("#do_inner##0#do_inner##1", "b", "captured", KnowsCaptured),
            ("#do_inner##2#do_inner##3", "x", "own", KnowsOwn),
            ("#do_inner##2#do_inner##3", "a", "captured", KnowsCaptured),
        ]
    );
    // Review B-076, 2: two definitions with closures on one line: Julia's
    // line cannot tell them apart, not compared.
    assert!(at(&l, 113).is_empty());
    let amb: Vec<usize> = l.ambiguous.iter().map(|a| a.1).collect();
    assert_eq!(amb, vec![113, 113]);
    assert_eq!(l.malformed, 0);
    let t = l.tally();
    assert_eq!(t.keys().copied().collect::<Vec<_>>(), vec![KnowsCaptured, KnowsOwn, KnowsGlobal, Undecided]);
    assert_eq!(l.rows.len(), 58);
}

/// WRONG (reported to Moonkale): a call's keyword argument name is in no
/// id. The shared normaliser reads `atol = x` inside a call's argument list
/// as an assignment to a local. Julia (1.13.1): `Clo.kw_a(0.5)` is false
/// and `Clo.kw_r(0.5)` true; `Clo.kw_x()` false and `Clo.kw_y()` true.
#[test]
fn keyword_names_are_in_no_id() {
    let l = ledger(&truth());
    let sites: Vec<(usize, &str)> = l.keywords_unspelled.iter().map(|k| (k.1, k.3.as_str())).collect();
    assert_eq!(sites, vec![(96, "atol"), (99, "atol"), (100, "rtol"), (101, "atol"), (102, "rtol")]);
    let (_, root) = sophia_ingest::module::package_root(Path::new(DIR)).unwrap();
    let r = sophia_ingest::module::ingest(&sophia_ingest::module::discover(&root).unwrap()).unwrap();
    let id = |name: &str| {
        r.module.definitions.iter().find(|x| x.name.as_deref() == Some(name)).and_then(|x| x.identity.as_ref().ok()).unwrap().digest
    };
    assert_eq!(id("kw_a"), id("kw_r"));
    assert_eq!(id("kw_x"), id("kw_y"));
}

/// Replaces column `col` of the truth line of closure `closure`.
fn with(truth: &str, closure: &str, col: usize, value: &str) -> String {
    truth
        .lines()
        .map(|l| {
            let mut c: Vec<&str> = l.split('\t').collect();
            if c.get(2) == Some(&closure) {
                c[col] = value;
            }
            c.join("\t") + "\n"
        })
        .collect()
}

/// The comparison reports each kind of wrong when Julia's side says so:
/// the fixture's truth with one closure's line falsified.
#[test]
fn the_comparison_reports_wrongs() {
    use ClosureOutcome::*;
    let t = truth();
    // `n` a global in Julia (Sophia: captured).
    let l = ledger(&with(&with(&t, "#adder##0#adder##1", 5, "-"), "#adder##0#adder##1", 7, "n=Clo.n"));
    assert_eq!(at(&l, 15)[1], ("n", "global", WrongLocal));
    // `scale` captured in Julia (Sophia: a module name).
    let l = ledger(&with(&t, "#scaled##0#scaled##1", 5, "scale"));
    assert_eq!(at(&l, 17)[1], ("scale", "captured", WrongGlobal));
    // `y` captured in Julia (Sophia: the closure's own).
    let l = ledger(&with(&t, "#fresh##0#fresh##1", 5, "y"));
    assert_eq!(at(&l, 25), vec![("y", "captured", WrongCapturedAsOwn)]);
    // `n` the closure's own in Julia (Sophia: captured).
    let l = ledger(&with(&with(&t, "#adder##0#adder##1", 5, "-"), "#adder##0#adder##1", 6, "n"));
    assert_eq!(at(&l, 15)[1], ("n", "own", WrongOwnAsCaptured));
}
