//! Experiment E-4 (macro expansions as transformation records) on the
//! fixture `julia-semantics/expansions`: Julia's expansion of each
//! module-level form that calls a macro (`expansions/truth.tsv`, printed by
//! `julia-semantics/expansions.jl`; `expansions/macros.tsv` by `macros.jl`;
//! see `expansions.md` to regenerate), spliced into the package and
//! ingested. Each case's expected outcome is the one [[Semantic Boundary
//! Experiments]] §E-4 records.

use std::path::Path;

use sophia_ingest::expansion::{ExpansionLedger, MethodOutcome, expansion_ledger, faithful, parse_expansions};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/julia-semantics/expansions");

fn truth() -> String {
    std::fs::read_to_string(Path::new(DIR).join("truth.tsv")).unwrap()
}

fn ledger() -> ExpansionLedger {
    expansion_ledger(Path::new(DIR), &truth()).unwrap()
}

fn outcomes(l: &ExpansionLedger, line: usize) -> Vec<(&str, MethodOutcome)> {
    l.rows.iter().filter(|r| r.line == line).map(|r| (r.owner.as_str(), r.outcome)).collect()
}

#[test]
fn expansions_on_the_fixture() {
    use MethodOutcome::*;
    let l = ledger();
    assert_eq!((l.forms, l.records), (20, 25), "{:?}", l.unmatched);
    // `shared.jl` is read into two modules: not compared.
    assert_eq!(l.unmatched, vec![("shared.jl".to_string(), 2)]);
    assert!(l.ambiguous.is_empty());
    // Every record's text parsed back and Julia evaluated it in place of the
    // form.
    assert!(l.not_ok.is_empty(), "{:?}", l.not_ok);
    // `@eval` loops: one record per value, each a plain definition with an
    // id; a qualified name spliced twice (`Base.$f`), types as values, a
    // splat.
    assert_eq!(outcomes(&l, 13), vec![("Base.sin", Recovered), ("Base.cos", Recovered)]);
    assert_eq!(outcomes(&l, 16), vec![("Exp.half", Recovered), ("Exp.half", Recovered)]);
    assert_eq!(outcomes(&l, 19), vec![("Exp.arity", Recovered), ("Exp.arity", Recovered)]);
    // Macros that make what the source as written does not show.
    assert_eq!(outcomes(&l, 43), vec![("Exp.tw", Recovered), ("Exp.tw_neg", Recovered)]);
    assert_eq!(outcomes(&l, 46), vec![("Exp.older", Recovered)]);
    let e = outcomes(&l, 47);
    assert_eq!(e.len(), 6, "{e:?}");
    assert!(e.iter().all(|(_, o)| *o == Recovered) && e.contains(&("Base.typemin", Recovered)));
    assert_eq!(outcomes(&l, 48), vec![("Exp.Opts", Recovered); 3]);
    // A macro around a definition, with or without a docstring: known as
    // written.
    assert_eq!(outcomes(&l, 53), vec![("Exp.inl", Known)]);
    assert_eq!(outcomes(&l, 56), vec![("Exp.docd", Known)]);
    // Docstrings made by a loop; `@static` around a loop.
    assert_eq!(outcomes(&l, 58), vec![("Exp.d1", Recovered), ("Exp.d2", Recovered)]);
    assert_eq!(outcomes(&l, 66), vec![("Exp.s1", Recovered), ("Exp.s2", Recovered)]);
    // `@eval Inner ex`: Julia makes `Exp.Inner.i1`; the text spliced into
    // `Exp` makes `Exp.i1` (approximates).
    assert_eq!(outcomes(&l, 76), vec![("Exp.Inner.i1", OtherModule)]);
    // A macro inside an `if`: not expanded, record 0; Sophia knows it.
    assert_eq!(outcomes(&l, 81), vec![("Exp.in_if", Known)]);
    // Bindings made by records (record 0's `in_if` is not one): all named by
    // a definition of the expansion (`@enum`'s type and constants too)
    // except `Exp.Inner.i1`, which the spliced text names `Exp.i1`.
    assert_eq!((l.bindings, l.bindings_covered), (22, 21));
}

#[test]
fn hidden_state_is_flagged() {
    // A closure as a splice's value does not read back from its `repr`; a
    // macro with a counter expands differently the second time. Every other
    // record is recomputed from its inputs to the same text.
    let l = ledger();
    let mut u = l.unstable.clone();
    u.sort();
    assert_eq!(u, vec![("Exp.jl".to_string(), 24, 1), ("Exp.jl".to_string(), 34, 1)]);
}

#[test]
fn addresses_determine_texts() {
    // E-4 fails if equal input addresses give different outputs. The
    // template is part of the address: several `@eval`s in one loop share
    // the form and the values.
    let l = ledger();
    let mut by: std::collections::HashMap<_, std::collections::HashSet<_>> = Default::default();
    for t in &l.transformations {
        by.entry(t.address).or_default().insert(t.text);
    }
    // One address per text, except `@stateful`'s two calls (review 1).
    assert_eq!(by.len(), l.transformations.len() - 1);
    // A package's own macro is identified by its definition: by its id if
    // it has one; `@twice`'s body is a `quote` with `$`, which the
    // normaliser refuses, so by its source text.
    let tw = l.transformations.iter().find(|t| t.line == 43).unwrap();
    assert!(tw.by == "Exp.@twice" && tw.by_id.is_none() && tw.by_text.is_some());
    let dep = l.transformations.iter().find(|t| t.line == 46).unwrap();
    assert!(dep.by == "Base.@deprecate" && dep.by_text.is_none() && dep.world == "1.13.1");
    // An eval record's parameters are the splices' values.
    let ar = l.transformations.iter().find(|t| t.line == 19 && t.k == 1).unwrap();
    assert_eq!(ar.parameters, vec!["(:a, :b)", "2"]);
}

#[test]
fn the_expansion_id_sees_the_macro() {
    // As written, `@inline inl(x) = x + 1` has the id of the bare definition
    // (part 5 of [[Julia Conformance]], wrong); the expansion carries the
    // `inline` flag, so its id differs from the one of the source as written.
    let l = ledger();
    let inl = l.transformations.iter().find(|t| t.line == 53).unwrap();
    assert_eq!(inl.output.len(), 1);
    let root = Path::new(DIR).join("src/Exp.jl");
    let d = sophia_ingest::module::discover(&root).unwrap();
    let r = sophia_ingest::module::ingest(&d).unwrap();
    let written = r.module.definitions.iter().find(|x| x.name.as_deref() == Some("inl")).unwrap();
    assert_ne!(written.identity.as_ref().unwrap().digest, inl.output[0].0);
}

#[test]
fn expansions_make_what_the_forms_make() {
    // The methods each form's records made (its text evaluated in its place)
    // are those the form makes as written (`macros.jl`), owner by owner.
    let m = std::fs::read_to_string(Path::new(DIR).join("macros.tsv")).unwrap();
    let f = faithful(&truth(), &m);
    assert_eq!(f.len(), 21);
    let differ: Vec<_> = f.iter().filter(|x| !x.2).collect();
    assert!(differ.is_empty(), "{differ:?}");
}

#[test]
fn parse_reads_escapes() {
    let tsv = "julia\t1.13.1\nrecord\tA.jl\t3\t1\teval\tCore.@eval\tA\tyes\tok\t1\t2\t3\t:a\\t1\tf() = $x\tf() = \\\"a\\\\n\\\"\\nend\n\
               method\tA.jl\t3\t1\tA.f\t()\n";
    let (v, r) = parse_expansions(tsv);
    assert_eq!(v, "1.13.1");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].inputs, vec![":a", "1"]);
    assert_eq!(r[0].text, "f() = \"a\\n\"\nend");
    assert_eq!(r[0].methods, vec![("A.f".to_string(), "()".to_string())]);
}

// Cases from the review of B-025 (Codex), each failing before its fix
// ([[Review 2026-11-19 Codex B-025 (external)]]).

#[test]
fn review_1_state_between_identical_calls_is_an_address_clash() {
    // `@stateful` reads a global that changes between two identical calls:
    // each call re-expands to the same text in its own state (`stable`), so
    // only the ledger sees it: one address, two texts.
    let l = ledger();
    let at = |line| l.transformations.iter().find(|t| t.line == line).unwrap();
    assert_eq!(at(91).address, at(93).address);
    assert_ne!(at(91).text, at(93).text);
    let mut c: Vec<usize> = l.clashes.iter().map(|x| x.1).collect();
    c.sort();
    assert_eq!(c, vec![91, 93]);
}

#[test]
fn review_2_a_splice_is_evaluated_once() {
    // `@eval sidefx() = $(SIDE[] += 1)`: Julia evaluates the splice once;
    // the recorded input is the value the definition got.
    let l = ledger();
    let t = l.transformations.iter().find(|t| t.line == 96).unwrap();
    assert_eq!(t.parameters, vec!["1"]);
    let (_, r) = parse_expansions(&truth());
    let r = r.iter().find(|r| r.line == 96).unwrap();
    assert_eq!(r.stable, Some(true));
    assert!(r.text.contains("    1\n"), "{}", r.text);
}

#[test]
fn review_3_quoted_gensyms_are_renamed() {
    // A gensym inside a quoted symbol is renamed like any other, so the text
    // does not depend on the process's gensym counter.
    let (_, r) = parse_expansions(&truth());
    let r = r.iter().find(|r| r.line == 101).unwrap();
    assert!(!r.text.contains("##"), "{}", r.text);
    assert_eq!(r.stable, Some(true));
}

#[test]
fn review_4_an_eval_inside_a_macro_expansion_is_spliced_once() {
    // `@mkeval five` expands to `@eval five() = 5`: the eval record holds
    // the definition, and the macro record's text keeps only its other
    // statements (none here), so the spliced package defines `five` once.
    let (_, r) = parse_expansions(&truth());
    let parent = r.iter().find(|r| r.line == 106 && r.kind == "macro").unwrap();
    assert!(!parent.text.contains("@eval"), "{}", parent.text);
    let l = ledger();
    let defs: usize = l.transformations.iter().filter(|t| t.line == 106).map(|t| t.output.len() + t.refused).sum();
    assert_eq!(defs, 1);
    assert_eq!(outcomes(&l, 106), vec![("Exp.five", MethodOutcome::Recovered)]);
}

#[test]
fn review_5_a_file_included_twice_lists_each_method_once() {
    // `shared.jl` is included into `A2` and `B2`; its record 0 lists each
    // module's method once, not both methods for both inclusions.
    let (_, r) = parse_expansions(&truth());
    let mut m: Vec<&str> = r.iter().filter(|r| r.file == "shared.jl").flat_map(|r| r.methods.iter().map(|x| x.0.as_str())).collect();
    m.sort();
    assert_eq!(m, vec!["Exp.A2.shared_f", "Exp.B2.shared_f"]);
}
