//! E-3 (B-024), dispatch facts and invalidation, on the fixture
//! `julia-semantics/invalidation`: each file in `changes/` adds one method
//! after `Inval`'s callers were compiled. `invalidation/truth/*.tsv` is what
//! `julia-semantics/dispatch_facts.jl` printed under Julia 1.13.1 (see
//! `dispatch_facts.md` to regenerate). Each test states, per change, what
//! Sophia's stale set must be against what changed and what Julia invalidated;
//! the expectations are the ledger of [[Dispatch Invalidation]].

use std::collections::BTreeSet;
use std::path::Path;

use sophia_ingest::invalidation::{Fact, FactKind, Rule, Run, Score};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/julia-semantics/invalidation/truth");

fn run(change: &str) -> Run {
    let text = std::fs::read_to_string(Path::new(DIR).join(format!("{change}.tsv"))).unwrap();
    Run::parse(&text).unwrap()
}

fn s(tp: usize, fp: usize, fn_: usize) -> Score {
    Score { tp, fp, fn_ }
}

/// The calls whose reachable methods changed, as "caller argument-types".
fn changed_calls(r: &Run) -> Vec<String> {
    let mut v: Vec<String> =
        r.facts.iter().filter(|f| f.kind != FactKind::Own && f.changed()).map(|f| format!("{} {:?}", short(&f.caller), f.sig)).collect();
    v.sort();
    v
}

/// The one recorded instance that prints as `printed` (names end in ` in <method id>`).
fn instance(r: &Run, printed: &str) -> String {
    let prefix = format!("MethodInstance for {printed} in ");
    let v: Vec<&String> = r.instances.keys().filter(|k| k.starts_with(&prefix)).collect();
    assert_eq!(v.len(), 1, "{printed}: {v:?}");
    v[0].clone()
}

fn short(mi: &str) -> &str {
    mi.trim_start_matches("MethodInstance for ")
}

const CHANGES: [&str; 12] = [
    "area_any",
    "area_circle_redefined",
    "area_shape_redefined",
    "area_shape_revised",
    "area_sq",
    "convert_meters_int",
    "first_or_int",
    "helper_h_int",
    "length_circle",
    "new_subtype",
    "perimeter_sq",
    "scale_int",
];

/// No rule may leave out a call that changed or an instance that relies on
/// one; and every instance Julia invalidated that a rule leaves out must be one
/// that relies on no changed fact (Julia invalidating more than changed).
#[test]
fn no_change_escapes_the_stale_set() {
    for c in CHANGES {
        let r = run(c);
        let rep = r.report();
        assert_eq!((rep.overlap.fn_, rep.specific.fn_), (0, 0), "{c}: a rule missed a changed call");
        assert_eq!(rep.julia_unrecorded, 0, "{c}: Julia invalidated an instance that was not recorded");
        let changed = r.affected(Fact::changed);
        let julia: BTreeSet<String> = r.instances.iter().filter(|(_, l)| **l).map(|(k, _)| k.clone()).collect();
        for rule in [Rule::Overlap, Rule::Specific] {
            let stale = r.affected(|f: &Fact| r.stale(f, rule));
            assert!(changed.is_subset(&stale), "{c} {rule:?}: {:?}", changed.difference(&stale).collect::<Vec<_>>());
            for m in julia.difference(&stale) {
                assert!(!changed.contains(m), "{c} {rule:?}: {m}");
            }
        }
    }
}

#[test]
fn a_more_specific_method_is_found_exactly() {
    // `area(::Sq)`: the `Sq`, `Shape` and untyped calls change, `Circle` does not.
    let r = run("area_sq");
    let rep = r.report();
    // Overlap also marks `invoke(area, Tuple{Shape}, c)`, which keeps `area(::Shape)`.
    assert_eq!((rep.overlap, rep.specific), (s(6, 1, 0), s(6, 0, 0)));
    assert_eq!((rep.julia, rep.specific_vs_julia), (18, s(18, 0, 0)));
    let calls = changed_calls(&r);
    assert!(calls.iter().any(|c| c.starts_with("Main.Inval.use_sq(")), "{calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("Main.Inval.use_circle(")), "{calls:?}");
    // `total` is invalidated only through Base's `mapreduce` chain: the edges carry it.
    let stale = r.affected(|f: &Fact| r.stale(f, Rule::Specific));
    assert!(stale.contains(&instance(&r, "Main.Inval.total(::Vector{Main.Inval.Shape})")));
}

#[test]
fn a_less_specific_method_needs_the_specificity_rule() {
    // `area(::Any)`: only the untyped call can reach it. Overlap alone marks
    // every `area` call; the specificity rule keeps exactly the one that changed.
    let rep = run("area_any").report();
    assert_eq!(rep.changed, 1);
    assert_eq!(rep.overlap, s(1, 7, 0));
    assert_eq!(rep.specific, s(1, 0, 0));
    assert_eq!(rep.julia, 2);
    assert_eq!(rep.overlap_vs_julia, s(2, 20, 0));
    assert_eq!(rep.specific_vs_julia, s(2, 0, 0));
}

#[test]
fn a_redefinition_marks_one_instance_julia_keeps() {
    // Replacing `area(::Circle)` changes what an `area(::Shape)` call can reach,
    // so the instance compiled for `area(::Shape)` (of the method `area(::Shape)`)
    // is stale for Sophia; Julia keeps it, since the replaced method was already
    // more specific than it and still is.
    let r = run("area_circle_redefined");
    let rep = r.report();
    assert_eq!(rep.specific, s(6, 0, 0));
    assert_eq!(rep.specific_vs_julia, s(17, 1, 0));
    let julia_keeps: Vec<&String> = r.instances.iter().filter(|(_, l)| !**l).map(|(k, _)| k).collect();
    let marked = r.affected(|f: &Fact| r.stale(f, Rule::Specific));
    let extra: Vec<&&String> = julia_keeps.iter().filter(|k| marked.contains(k.as_str())).collect();
    assert_eq!(extra.len(), 1);
    assert!(extra[0].starts_with("MethodInstance for Main.Inval.area(::Main.Inval.Shape) in Main.Inval.area@Inval.jl:17#"), "{extra:?}");
}

#[test]
fn the_instance_invoke_makes_is_its_own() {
    // `invoke(area, Tuple{Shape}, c)` with a `Circle` makes an instance of the
    // method `area(::Shape)` for `Circle`, which prints like the instance of
    // `area(::Circle)`. Kept apart, Julia and the specificity rule agree exactly
    // when `area(::Shape)` is replaced.
    for c in ["area_shape_redefined", "area_shape_revised"] {
        let r = run(c);
        let rep = r.report();
        assert_eq!(rep.specific, s(7, 0, 0), "{c}");
        assert_eq!(rep.specific_vs_julia, s(20, 0, 0), "{c}");
        let circles: Vec<&String> =
            r.instances.keys().filter(|k| k.starts_with("MethodInstance for Main.Inval.area(::Main.Inval.Circle) in ")).collect();
        assert_eq!(circles.len(), 2, "{c}: {circles:?}");
    }
}

#[test]
fn a_new_subtype_changes_only_abstract_calls() {
    let r = run("new_subtype");
    let rep = r.report();
    assert_eq!(rep.specific, s(5, 0, 0));
    assert_eq!(rep.specific_vs_julia, s(16, 0, 0));
    let calls = changed_calls(&r);
    assert!(!calls.iter().any(|c| c.starts_with("Main.Inval.use_circle(") || c.starts_with("Main.Inval.use_sq(")), "{calls:?}");
}

#[test]
fn parameters_and_type_arguments_are_told_apart() {
    // `scale(::Vector{Int})` does not touch the `Vector{Float64}` call;
    // `first_or(::Vector{Int}, ::Int)` is more specific than a `where` method;
    // `convert(::Type{Meters}, ::Int)` does not touch the `Float64` call.
    for (c, changed, julia) in [("scale_int", 1, 3), ("first_or_int", 1, 3), ("convert_meters_int", 0, 0)] {
        let rep = run(c).report();
        assert_eq!(rep.changed, changed, "{c}");
        assert_eq!(rep.overlap, s(changed, 0, 0), "{c}");
        assert_eq!(rep.overlap_vs_julia, s(julia, 0, 0), "{c}");
    }
}

#[test]
fn methods_of_other_functions_change_nothing() {
    for c in ["perimeter_sq", "length_circle"] {
        let rep = run(c).report();
        assert_eq!((rep.changed, rep.julia), (0, 0), "{c}");
        assert_eq!(rep.overlap, s(0, 0, 0), "{c}");
        assert_eq!(rep.overlap_vs_julia, s(0, 0, 0), "{c}");
    }
}

#[test]
fn an_invoke_call_is_a_fact() {
    // Codex's review: `invoke(area, Tuple{Shape}, c)` is a builtin call and was
    // skipped, so nothing marked `use_invoke` when `area(::Shape)` was replaced.
    let r = run("area_shape_redefined");
    let invoke = instance(&r, "Main.Inval.use_invoke(::Main.Inval.Circle)");
    assert_eq!(r.instances.get(&invoke), Some(&true), "Julia invalidates it");
    assert!(r.facts.iter().any(|f| f.caller == invoke && f.kind == FactKind::Invoke && f.changed()), "its invoke call changed");
    assert!(r.affected(|f: &Fact| r.stale(f, Rule::Specific)).contains(&invoke));
}

#[test]
fn a_method_replaced_at_its_own_location_changes_its_calls() {
    // Codex's review: methods were named by file and line, so a method
    // re-evaluated at its own location (as Revise does) looked unchanged.
    let rep = run("area_shape_revised").report();
    assert_eq!(rep.changed, 7, "{rep:?}");
    assert_eq!(rep.specific.fn_, 0);
}

#[test]
fn an_invoke_call_is_followed_to_the_invoked_instance() {
    // Codex's second review: the invoked instance was not followed, so a method
    // added inside it (`Helper.h(::Int)`, called by `Helper.g`) never reached the
    // caller of `invoke(Helper.g, Tuple{Number}, x)`.
    let r = run("helper_h_int");
    let caller = instance(&r, "Main.Inval.use_helper(::Int64)");
    assert_eq!(r.instances.get(&caller), Some(&true), "Julia invalidates it");
    assert!(r.affected(|f: &Fact| r.stale(f, Rule::Specific)).contains(&caller));
}
