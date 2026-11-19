//! Part 5 of the Julia conformance suite (macros), on the fixture
//! `julia-semantics/macros`: for each module-level form that calls a macro,
//! what Julia's form made (`macros/truth.tsv`, printed by
//! `julia-semantics/macros.jl`; see `macros.md` to regenerate) against the
//! definitions Sophia records in it, and whether Sophia's id sees a macro
//! that wraps a definition. Each case's expected outcome is the ledger entry
//! of [[Julia Conformance]] part 5.

use std::path::Path;

use sophia_ingest::conformance::{MacroLedger, MacroOutcome, macro_ledger};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/julia-semantics/macros");

fn ledger() -> MacroLedger {
    let truth = std::fs::read_to_string(Path::new(DIR).join("truth.tsv")).unwrap();
    macro_ledger(Path::new(DIR), &truth).unwrap()
}

fn outcomes(l: &MacroLedger, line: usize) -> Vec<(&str, MacroOutcome)> {
    l.rows.iter().filter(|r| r.line == line).map(|r| (r.name.as_str(), r.outcome)).collect()
}

#[test]
fn macro_ledger_on_the_fixture() {
    use MacroOutcome::*;
    let l = ledger();
    assert_eq!(l.forms, 23, "{:?}", l.unmatched);
    assert!(l.unmatched.is_empty());
    // WRONG (reported to Moonkale): a macro around a definition changes the
    // method Julia compiles (inlining, bounds checks, a generator instead of
    // a body, a keyword constructor), and Sophia's id is the same without
    // it: the definition's span starts after the macro.
    for (line, name) in [(22, "Mac.inl"), (23, "Mac.noinl"), (26, "Mac.getit"), (27, "Mac.gen"), (35, "Mac.defarg")] {
        assert_eq!(outcomes(&l, line), vec![(name, KnowsDefinition), (name, IdIgnoresMacro)], "line {line}");
    }
    assert_eq!(outcomes(&l, 43), vec![("Mac.K", KnowsDefinition), ("Mac.K", IdIgnoresMacro), ("Mac.K", OpenConstructors)]);
    // A macro that changes nothing, and `@eval` without `$`: one id is right.
    assert_eq!(outcomes(&l, 32), vec![("Mac.kept", KnowsDefinition), ("Mac.kept", IdNoChange)]);
    assert_eq!(outcomes(&l, 57), vec![("Mac.evald", KnowsDefinition), ("Mac.evald", IdNoChange)]);
    // Definitions only the expansion has: left open, wherever Julia files
    // them (`@twice`'s in the macro's body, `@enum`'s on Base's functions).
    assert_eq!(outcomes(&l, 38), vec![("Mac.tw", Open), ("Mac.tw_neg", Open)]);
    assert_eq!(outcomes(&l, 39), vec![("Mac.old", Open)]);
    let e = outcomes(&l, 40);
    assert_eq!(e.len(), 8, "{e:?}");
    assert!(e.iter().all(|(_, o)| *o == Open));
    assert!(e.contains(&("Mac.Color", Open)) && e.contains(&("Mac.red", Open)) && e.contains(&("Base.typemin", Open)));
    // `@eval` loops: one definition, a method per value.
    assert_eq!(outcomes(&l, 49), vec![("$n() = $v", Template)]);
    assert_eq!(outcomes(&l, 52), vec![("Mac.tmpl", Template)]);
    // `@static`: the branch not taken is a definition Julia never makes.
    assert_eq!(outcomes(&l, 60), vec![("Mac.plat", NotMade), ("Mac.plat", KnowsDefinition)]);
    // A docstring: the definition, no id comparison (documentation is not
    // the method).
    assert_eq!(outcomes(&l, 67), vec![("Mac.documented", KnowsDefinition)]);
    // Cases from the review of B-074 (Mistral), each failing before its fix.
    // Two forms on one line: not compared (finding 2).
    assert_eq!(l.ambiguous, vec![("Mac.jl".to_string(), 72)]);
    assert!(outcomes(&l, 72).is_empty());
    // Blanking a non-ASCII macro argument moves no offset (finding 6): the
    // macro that changes nothing keeps one id, the next form is unaffected.
    assert_eq!(outcomes(&l, 77), vec![("Mac.uni", KnowsDefinition), ("Mac.uni", IdNoChange)]);
    assert_eq!(outcomes(&l, 78), vec![("Mac.uni_after", KnowsDefinition), ("Mac.uni_after", IdIgnoresMacro)]);
    // A parenthesised argument is compared for what it makes, not for its id.
    assert_eq!(outcomes(&l, 80), vec![("Mac.paren", KnowsDefinition)]);
    // A constructor of a type parameter (finding 4: the oracle dropped it).
    let tv = outcomes(&l, 83);
    assert_eq!(tv.iter().map(|x| x.1).collect::<Vec<_>>(), vec![KnowsDefinition, IdIgnoresMacro]);
    // A docstring on a module does not move the next form (finding 9); a
    // `baremodule` is followed like a module.
    assert_eq!(outcomes(&l, 87), vec![("Mac.Dm.dm", KnowsDefinition), ("Mac.Dm.dm", IdIgnoresMacro)]);
    assert_eq!(outcomes(&l, 89), vec![("Mac.after_dm", KnowsDefinition), ("Mac.after_dm", IdIgnoresMacro)]);
    assert_eq!(outcomes(&l, 93), vec![("Mac.Bm.bm", KnowsDefinition), ("Mac.Bm.bm", IdIgnoresMacro)]);
    // `@eval` with a splice is a template, not compared for its id (suspect 11).
    assert_eq!(outcomes(&l, 96).iter().map(|x| x.1).collect::<Vec<_>>(), vec![Template]);
    // Every method Julia made is accounted for.
    assert_eq!((l.methods_seen, l.methods_open), (34, 12));
    let wrong = l.rows.iter().filter(|r| r.outcome == IdIgnoresMacro).count();
    assert_eq!(wrong, 11);
}

/// A package directory written relatively compares the same forms (review
/// B-074, finding 1: it compared none).
#[test]
fn relative_package_dir() {
    let truth = std::fs::read_to_string(Path::new(DIR).join("truth.tsv")).unwrap();
    let rel = Path::new("tests/julia-semantics/macros"); // cargo runs tests in the crate's directory
    let l = macro_ledger(rel, &truth).unwrap();
    assert_eq!((l.forms, l.unmatched.len()), (23, 0));
}

/// The wrong above, without the oracle: the same definition with and without
/// `@generated` gets one id, though Julia runs the body in one case and
/// returns it as code in the other.
#[test]
fn generated_and_plain_share_an_id() {
    use sophia_ingest::module::{Discovered, SourceFile, ingest};
    let id = |text: &str| {
        let d = Discovered {
            files: vec![SourceFile { path: "G.jl".into(), text: text.into(), base_module: vec!["G".into()] }],
            dynamic: Vec::new(),
        };
        let r = ingest(&d).unwrap();
        r.module.definitions.iter().find(|x| x.name.as_deref() == Some("gen")).unwrap().identity.as_ref().unwrap().digest
    };
    assert_eq!(id("@generated function gen(x)\n    :(x)\nend\n"), id("function gen(x)\n    :(x)\nend\n"));
}
