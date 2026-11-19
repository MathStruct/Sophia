//! Part 4 of the Julia conformance suite (world change), on the fixture
//! `julia-semantics/world`: each file in `src/changes/` (evaluated into the
//! package, ingested by Sophia) and `outside/` (evaluated into `Main`, seen
//! only by Julia) adds a method or an import after the package is loaded.
//! The part 3 comparison runs before and after (`world/truth/*.tsv`, printed
//! by `julia-semantics/calls.jl`; see `calls.md` to regenerate), and each
//! of Sophia's facts is classified by what the change did to it. Each case's
//! expected outcome is the ledger entry of [[Julia Conformance]] part 4.

use std::path::Path;

use sophia_ingest::conformance::{Outcome, WorldLedger, WorldOutcome, world_ledger};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/julia-semantics/world");

fn ledger(change: &str) -> WorldLedger {
    let read = |name: &str| std::fs::read_to_string(Path::new(DIR).join("truth").join(format!("{name}.tsv"))).unwrap();
    let stem = Path::new(change).file_stem().unwrap().to_str().unwrap();
    world_ledger(Path::new(DIR), &Path::new(DIR).join(change), &read("base"), &read(stem)).unwrap()
}

fn row(w: &WorldLedger, caller: &str, name: &str) -> (Option<Outcome>, Option<Outcome>, WorldOutcome) {
    let r: Vec<_> = w.rows.iter().filter(|r| r.caller == caller && r.name == name).collect();
    assert_eq!(r.len(), 1, "{caller} → {name}: {r:?}");
    (r[0].before, r[0].after, r[0].outcome)
}

fn moved(w: &WorldLedger) -> Vec<&str> {
    let mut v: Vec<&str> = w.ids_moved.iter().map(|(_, _, c)| c.as_str()).collect();
    v.sort_unstable();
    v
}

/// Every row not named in `expected` stays as it was (true, open, or part
/// 3's wrong); `IdMoved` only for a caller in `ids_moved`.
fn rest_unchanged(w: &WorldLedger, expected: &[(&str, &str)]) {
    for r in &w.rows {
        if expected.contains(&(r.caller.as_str(), r.name.as_str())) {
            continue;
        }
        let moved = w.ids_moved.iter().any(|(f, l, c)| *f == r.file && *l == r.line && *c == r.caller);
        if moved && r.outcome == WorldOutcome::IdMoved {
            continue;
        }
        assert!(matches!(r.outcome, WorldOutcome::StillTrue | WorldOutcome::StillOpen | WorldOutcome::StillWrong), "{r:?}");
    }
}

#[test]
fn a_method_added_to_a_function_with_several() {
    // `call_duo` uses the function; its id does not mention a method, so a
    // third method changes nothing Sophia states (SP-006's answer).
    for change in ["src/changes/duo_more.jl", "outside/duo_elsewhere.jl"] {
        let w = ledger(change);
        assert_eq!(row(&w, "call_duo", "duo"), (Some(Outcome::KnowsFunction), Some(Outcome::KnowsFunction), WorldOutcome::StillTrue));
        assert!(w.ids_moved.is_empty(), "{change}: {:?}", w.ids_moved);
        rest_unchanged(&w, &[]);
    }
}

#[test]
fn a_second_method_in_the_package_moves_ids() {
    // Sophia follows: `call_solo` now uses the function. But its id moves
    // with it, and so does `twice`'s, which refers to `call_solo` by content
    // and whose own claim did not change (approximates: over-invalidation).
    let w = ledger("src/changes/solo_more.jl");
    assert!(w.ingested);
    assert_eq!(row(&w, "call_solo", "solo"), (Some(Outcome::KnowsOnlyDefinition), Some(Outcome::KnowsFunction), WorldOutcome::Followed));
    assert_eq!(
        row(&w, "twice", "call_solo"),
        (Some(Outcome::KnowsOnlyDefinition), Some(Outcome::KnowsOnlyDefinition), WorldOutcome::IdMoved)
    );
    assert_eq!(moved(&w), vec!["call_solo", "twice"]);
    rest_unchanged(&w, &[("call_solo", "solo"), ("twice", "call_solo")]);
}

#[test]
fn a_second_method_from_another_package_makes_a_fact_stale() {
    // WRONG (reported to Moonkale): Sophia ingests only `World` and cannot
    // see the method; `call_solo`'s id still says it reaches `solo(x)`.
    let w = ledger("outside/solo_elsewhere.jl");
    assert!(!w.ingested);
    assert_eq!(row(&w, "call_solo", "solo"), (Some(Outcome::KnowsOnlyDefinition), Some(Outcome::WrongOneOfMany), WorldOutcome::MadeStale));
    assert!(w.ids_moved.is_empty());
    rest_unchanged(&w, &[("call_solo", "solo")]);
}

#[test]
fn a_qualified_method_from_the_parent_makes_a_fact_stale() {
    // WRONG (part 3's first wrong, reported to Moonkale): ingested, yet the
    // crate looks only at `Inner` and keeps the content reference.
    let w = ledger("src/changes/inner_h.jl");
    assert_eq!(row(&w, "call_h", "h"), (Some(Outcome::KnowsOnlyDefinition), Some(Outcome::WrongOneOfMany), WorldOutcome::MadeStale));
    assert!(w.ids_moved.is_empty());
    rest_unchanged(&w, &[("call_h", "h")]);
}

#[test]
fn a_local_function_shadowing_base() {
    // Julia >= 1.12: from the next world `call_sum` calls `World.sum`.
    // Sophia, re-ingesting, refers to it by content.
    let w = ledger("src/changes/shadow_sum.jl");
    assert_eq!(row(&w, "call_sum", "sum"), (Some(Outcome::OpenOutside), Some(Outcome::KnowsOnlyDefinition), WorldOutcome::Followed));
    assert_eq!(moved(&w), vec!["call_sum"]);
    rest_unchanged(&w, &[("call_sum", "sum")]);
}

#[test]
fn an_import_then_a_method() {
    // `Base.maximum` gains a method here; `call_max` used it before and
    // after, and Sophia states nothing about it either way.
    let w = ledger("src/changes/import_max.jl");
    let (_, after, outcome) = row(&w, "call_max", "maximum");
    assert_eq!((after, outcome), (Some(Outcome::OpenHere), WorldOutcome::StillOpen));
    assert!(w.ids_moved.is_empty());
    rest_unchanged(&w, &[]);
}

#[test]
fn a_change_that_includes_a_file() {
    // Review B-073 (Codex 1): the file a change includes is the package's
    // code too; Sophia must ingest it, as for `solo_more.jl`.
    let w = ledger("src/changes/via_include.jl");
    assert_eq!(row(&w, "call_solo", "solo"), (Some(Outcome::KnowsOnlyDefinition), Some(Outcome::KnowsFunction), WorldOutcome::Followed));
    assert_eq!(moved(&w), vec!["call_solo", "twice"]);
    rest_unchanged(&w, &[("call_solo", "solo"), ("twice", "call_solo")]);
}

#[test]
fn the_result_does_not_depend_on_how_paths_are_written() {
    // Review B-073 (Codex 2): a relative package directory with an
    // absolute change path gave `MadeStale` for `call_sum`.
    let rel = Path::new("tests/julia-semantics/world");
    let read = |name: &str| std::fs::read_to_string(rel.join("truth").join(format!("{name}.tsv"))).unwrap();
    let w = world_ledger(rel, &Path::new(DIR).join("src/changes/shadow_sum.jl"), &read("base"), &read("shadow_sum")).unwrap();
    assert_eq!(row(&w, "call_sum", "sum"), (Some(Outcome::OpenOutside), Some(Outcome::KnowsOnlyDefinition), WorldOutcome::Followed));
}

#[test]
fn a_method_replaced_from_outside_is_unchecked() {
    // Review B-073 (Codex 3): Julia lists no package method at `call_solo`'s
    // line after another package replaced it; Sophia still states the same
    // reference. The oracle stopped checking it: not `Followed`.
    let w = ledger("outside/redefine_call_solo.jl");
    let (before, after, outcome) = row(&w, "call_solo", "solo");
    assert_eq!((before, after, outcome), (Some(Outcome::KnowsOnlyDefinition), None, WorldOutcome::Unchecked));
}
