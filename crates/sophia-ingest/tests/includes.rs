//! Part 9 of the Julia conformance suite (include order), on the fixture
//! `julia-semantics/includes`: which files Sophia's include discovery
//! (`module::discover`) reads, in which order and into which module, against
//! the files Julia evaluates when it loads the package, and where Sophia puts
//! each of Julia's methods. Julia's side is `includes/truth.tsv`, printed by
//! `julia-semantics/includes.jl` (see `includes.md` to regenerate). Each
//! case's expected outcome is the ledger entry of [[Julia Conformance]]
//! part 9.

use std::path::Path;

use sophia_ingest::conformance::{IncludeLedger, IncludeOutcome, PlaceOutcome, include_ledger};
use sophia_ingest::module;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/julia-semantics/includes");

fn truth() -> String {
    std::fs::read_to_string(Path::new(DIR).join("truth.tsv")).unwrap()
}

fn file<'l>(l: &'l IncludeLedger, f: &str) -> (Vec<&'l str>, Vec<&'l str>, IncludeOutcome) {
    let r = l.files.iter().find(|r| r.file == f).unwrap_or_else(|| panic!("no file {f}"));
    (r.julia.iter().map(String::as_str).collect(), r.sophia.iter().map(String::as_str).collect(), r.outcome)
}

fn method(l: &IncludeLedger, f: &str, module: &str) -> PlaceOutcome {
    l.methods.iter().find(|r| r.file == f && r.module == module).unwrap_or_else(|| panic!("no method in {f} of {module}")).outcome
}

#[test]
fn include_ledger_on_the_fixture() {
    use IncludeOutcome::*;
    let l = include_ledger(Path::new(DIR), &truth()).unwrap();
    assert_eq!(l.malformed, 0);

    // Literal includes, relative to the including file, into the module
    // around the call: Sophia reads them as Julia does.
    assert_eq!(file(&l, "Inc.jl"), (vec![""], vec![""], Knows));
    assert_eq!(file(&l, "a.jl"), (vec!["Inc"], vec!["Inc"], Knows));
    assert_eq!(file(&l, "sub/b.jl"), (vec!["Inc"], vec!["Inc"], Knows));
    assert_eq!(file(&l, "sub/c.jl"), (vec!["Inc"], vec!["Inc"], Knows));
    assert_eq!(file(&l, "sub/deep.jl"), (vec!["Inc.Deep"], vec!["Inc.Deep"], Knows));
    assert_eq!(file(&l, "inner.jl"), (vec!["Inc.Inner"], vec!["Inc.Inner"], Knows));
    // Included twice into one module: read once, the same methods.
    assert_eq!(file(&l, "again.jl"), (vec!["Inc", "Inc"], vec!["Inc"], ApproxRepeated));
    // A path not in normal form: the same file, Sophia keeps the spelling.
    assert_eq!(file(&l, "norm.jl"), (vec!["Inc"], vec!["Inc"], ApproxPath));
    // Dynamic includes in `Dyn`: Sophia records them, so `Dyn` stays open.
    for f in ["dyn/joined.jl", "dyn/looped.jl", "dyn/mapped.jl"] {
        assert_eq!(file(&l, f), (vec!["Inc.Dyn"], vec![], OpenDynamic), "{f}");
    }

    // A file included into two modules is read once per module (until
    // 2026-11-19 it was read once, into the first: wrong).
    assert_eq!(file(&l, "twice.jl"), (vec!["Inc", "Inc.Other"], vec!["Inc", "Inc.Other"], Knows));
    // WRONG: `Inc.include(…)` and `Base.include(Inc, …)` are no include to
    // Sophia, which records no dynamic include in `Inc`.
    assert_eq!(file(&l, "qualified.jl"), (vec!["Inc"], vec![], WrongMissed));
    assert_eq!(file(&l, "baseinc.jl"), (vec!["Inc"], vec![], WrongMissed));
    // WRONG: an include Julia never evaluates at load (`@static if false`, a
    // function nobody calls, quoted code) is read as part of `Inc`.
    for f in ["never_static.jl", "never_called.jl", "never_quoted.jl"] {
        assert_eq!(file(&l, f), (vec![], vec!["Inc"], WrongExtra), "{f}");
    }
    assert_eq!(l.extra_definitions, 3);
    // An include in a function body called at load: the file is Julia's too,
    // and in the module Sophia says, but evaluated last.
    assert_eq!(file(&l, "late.jl"), (vec!["Inc"], vec!["Inc"], Knows));
    assert_eq!(l.moved, vec!["late.jl".to_string()]);
    assert_eq!(l.files.len(), 18);

    // Methods: placed where Julia defines them, except in the wrong files.
    use PlaceOutcome as P;
    assert_eq!(method(&l, "twice.jl", "Inc"), P::Knows);
    assert_eq!(method(&l, "twice.jl", "Inc.Other"), P::Knows);
    assert_eq!(method(&l, "sub/deep.jl", "Inc.Deep"), P::Knows);
    assert_eq!(method(&l, "qualified.jl", "Inc"), P::WrongMissed);
    assert_eq!(method(&l, "dyn/mapped.jl", "Inc.Dyn"), P::OpenDynamic);
    let t = l.method_tally();
    assert_eq!((t[&P::Knows], t.get(&P::WrongModule), t[&P::WrongMissed], t[&P::OpenDynamic]), (16, None, 2, 3));
    assert_eq!(l.methods_outside, 0);
}

/// What a missed file does to ids: `shared` has a method in `qualified.jl`
/// that Sophia does not read, so `use_shared` refers by content to the one
/// method Sophia sees (part 3's `WrongOneOfMany`), while Julia dispatches
/// `use_shared("x")` to the other.
#[test]
fn a_missed_file_makes_a_content_reference() {
    let (_, root) = module::package_root(Path::new(DIR)).unwrap();
    let d = module::discover(&root).unwrap();
    let m = module::ingest(&d).unwrap().module;
    let def = |name: &str| m.definitions.iter().find(|x| x.name.as_deref() == Some(name)).unwrap();
    let shared = def("shared").key;
    assert_eq!(m.definitions.iter().filter(|x| x.name.as_deref() == Some("shared")).count(), 1);
    assert!(def("use_shared").references.iter().any(|(_, t)| *t == shared));
}

/// An include of a file that does not exist makes discovery fail for the
/// whole package, even when Julia never evaluates it (here: a branch
/// `@static` removes). Sophia leaves the package open; Julia loads it.
#[test]
fn a_missing_file_in_a_dead_branch_refuses_the_package() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("Project.toml"), "name = \"Gone\"\n").unwrap();
    std::fs::write(dir.path().join("src/Gone.jl"), "module Gone\n@static if false\n    include(\"missing.jl\")\nend\nf() = 1\nend\n")
        .unwrap();
    let (_, root) = module::package_root(dir.path()).unwrap();
    assert!(matches!(module::discover(&root), Err(module::ModuleError::Io(_))));
}

#[test]
fn malformed_truth_is_counted() {
    let mut t = truth();
    t.push_str("include\tx\tInc\ta.jl\nmethod\ta.jl\tone\tInc\tf\nnonsense\n");
    let l = include_ledger(Path::new(DIR), &t).unwrap();
    assert_eq!(l.malformed, 3);
}

/// Falsified truth: each kind of disagreement is found when Julia's side
/// says so, so the fixture's outcomes are not an artefact of the comparison.
#[test]
fn falsified_truth_gives_each_outcome() {
    use IncludeOutcome::*;
    let t = truth()
        // a.jl evaluated last, in another module, its method there too;
        .replace("include\t1\tInc\ta.jl", "include\t99\tInc.Inner\ta.jl")
        .replace("method\ta.jl\t1\tInc\ta_f", "method\ta.jl\t1\tInc.Inner\ta_f")
        // inner.jl evaluated a second time, into the same module;
        .replace("include\t16\tInc\tlate.jl", "include\t16\tInc\tlate.jl\ninclude\t17\tInc.Inner\tinner.jl")
        // never_static.jl evaluated after all, at the end.
        .replace("include\t0\t-\tInc.jl", "include\t0\t-\tInc.jl\ninclude\t18\tInc\tnever_static.jl");
    let l = include_ledger(Path::new(DIR), &t).unwrap();
    assert_eq!(l.malformed, 0);
    assert_eq!(file(&l, "a.jl").2, WrongModule);
    assert_eq!(method(&l, "a.jl", "Inc.Inner"), PlaceOutcome::WrongModule);
    assert_eq!(file(&l, "inner.jl").2, ApproxRepeated);
    assert_eq!(file(&l, "never_static.jl").2, Knows);
    assert_eq!(l.extra_definitions, 2);
    // Julia now evaluates a.jl after late.jl and never_static.jl; Sophia
    // reads it third.
    let mut moved = l.moved.clone();
    moved.sort();
    assert_eq!(moved, vec!["a.jl".to_string(), "late.jl".to_string()]);
}

/// Review B-083 (Codex, 3): a file evaluated into a module with a dynamic
/// include and also into one without: its methods in the first stay open,
/// whatever the file's row says.
#[test]
fn a_method_is_open_by_its_own_module() {
    let t = truth().replace("include\t16\tInc\tlate.jl", "include\t16\tInc\tlate.jl\ninclude\t17\tInc\tdyn/joined.jl");
    let l = include_ledger(Path::new(DIR), &t).unwrap();
    assert_eq!(file(&l, "dyn/joined.jl").2, IncludeOutcome::WrongMissed);
    assert_eq!(method(&l, "dyn/joined.jl", "Inc.Dyn"), PlaceOutcome::OpenDynamic);
}

/// Review B-083 (Codex, 4): one file reached by two spellings (a symbolic
/// link) is one file: Julia evaluates it twice into `P`, Sophia reads it
/// once. Julia's truth for this package (`julia includes.jl P src`): both
/// includes, and the method at the second spelling, which redefined it.
#[cfg(unix)]
#[test]
fn a_symbolic_link_is_the_same_file() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(dir.path().join("Project.toml"), "name = \"P\"\n").unwrap();
    std::fs::write(src.join("P.jl"), "module P\ninclude(\"a.jl\")\ninclude(\"alias.jl\")\nend\n").unwrap();
    std::fs::write(src.join("a.jl"), "f() = 1\n").unwrap();
    std::os::unix::fs::symlink("a.jl", src.join("alias.jl")).unwrap();
    let truth = "include\t0\t-\tP.jl\ninclude\t1\tP\ta.jl\ninclude\t2\tP\talias.jl\nmethod\talias.jl\t1\tP\tf\n";
    let l = include_ledger(dir.path(), truth).unwrap();
    assert_eq!(l.files.len(), 2, "{l:?}");
    assert_eq!(file(&l, "a.jl"), (vec!["P", "P"], vec!["P"], IncludeOutcome::ApproxRepeated));
    assert_eq!(method(&l, "a.jl", "P"), PlaceOutcome::Knows);
}

/// Review B-083 (Mistral, 1 to 3): a method in `Main` (printed `-`, as the
/// root file's module) is compared, not malformed; a method at a line no
/// definition covers is counted apart; a file written not in normal form is
/// the same key; a duplicate `seq` is malformed.
#[test]
fn edge_lines_of_the_truth() {
    let t = truth()
        + "method\tInc.jl\t5\t-\troot_first\n"
        + "method\tInc.jl\t3\tInc\tnowhere\n"
        + "method\tsub/../a.jl\t1\tInc.Inner\ta_f\n"
        + "include\t1\tInc\ta.jl\n";
    let l = include_ledger(Path::new(DIR), &t).unwrap();
    assert_eq!(l.malformed, 1);
    let main = l.methods.iter().find(|r| r.module.is_empty()).expect("the method in Main");
    assert_eq!(main.outcome, PlaceOutcome::WrongModule);
    assert_eq!(l.methods_outside, 1);
    assert_eq!(method(&l, "a.jl", "Inc.Inner"), PlaceOutcome::WrongModule);
}
