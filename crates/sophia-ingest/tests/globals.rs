//! Part 7 of the Julia conformance suite (mutable globals), on the fixture
//! `julia-semantics/globals`: for each global binding of the package that
//! holds data, whether Sophia gives it a definition with an id, and for each
//! use of one in a method, whether Sophia's id treats it as content (a
//! content reference), as a generic function, or as state (only the
//! spelling). Julia's side is `globals/truth.tsv`, printed by
//! `julia-semantics/globals.jl` (see `globals.md` to regenerate). Each
//! case's expected outcome is the ledger entry of [[Julia Conformance]]
//! part 7.

use std::path::Path;

use sophia_ingest::conformance::{GlobalLedger, GlobalOutcome, global_ledger};
use sophia_store::Store;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/julia-semantics/globals");

fn truth() -> String {
    std::fs::read_to_string(Path::new(DIR).join("truth.tsv")).unwrap()
}

fn ledger(truth: &str) -> GlobalLedger {
    global_ledger(Path::new(DIR), truth).unwrap()
}

/// (binding, access, Julia's kind, outcome) of the rows at a line.
fn at(l: &GlobalLedger, line: usize) -> Vec<(&str, &str, &str, GlobalOutcome)> {
    l.rows.iter().filter(|r| r.line == line).map(|r| (r.binding.as_str(), r.access.as_str(), r.julia.as_str(), r.outcome)).collect()
}

fn binding<'l>(l: &'l GlobalLedger, b: &str) -> (&'l str, Vec<&'l str>, GlobalOutcome) {
    let r = l.bindings.iter().find(|r| r.binding == b).unwrap_or_else(|| panic!("no binding {b}"));
    (r.julia.as_str(), r.sophia.iter().map(String::as_str).collect(), r.outcome)
}

#[test]
fn global_ledger_on_the_fixture() {
    use GlobalOutcome::*;
    let l = ledger(&truth());
    assert_eq!(l.malformed, 0);
    assert!(l.outside.is_empty(), "{l:?}");

    // Bindings. A constant is a `const` definition with an id; an untyped
    // mutable global (`scale = 2`, assigned twice, declared by `global later`
    // and assigned only by a function, assigned only by a function, in a
    // submodule) is no definition: state.
    assert_eq!(binding(&l, "Glob.LIMIT"), ("const", vec!["const"], KnowsConstant));
    assert_eq!(binding(&l, "Glob.STEP"), ("const", vec!["const", "const"], KnowsConstant));
    assert_eq!(binding(&l, "Glob.Inner.WIDTH"), ("const", vec!["const"], KnowsConstant));
    assert_eq!(binding(&l, "Glob.DOUBLE"), ("const", vec!["const"], KnowsConstant));
    // Left open: constants an `@enum` makes have no definition.
    assert_eq!(binding(&l, "Glob.RED"), ("const", vec![], OpenConstant));
    for b in ["Glob.scale", "Glob.counter", "Glob.mode", "Glob.later", "Glob.fresh", "Glob.Inner.depth"] {
        assert_eq!(binding(&l, b), ("global", vec![], KnowsState), "{b}");
    }
    // WRONG: a typed global `level::Int = 1` is read as the short form of a
    // function definition (`f(x)::T = …`), so it gets an id.
    assert_eq!(binding(&l, "Glob.level"), ("global", vec!["assignment"], WrongContentOfMutable));
    assert_eq!(binding(&l, "Glob.mode2"), ("global", vec!["assignment", "assignment"], WrongContentOfMutable));
    assert_eq!(l.bindings.len(), 20);

    // Uses. A constant: a content reference.
    assert_eq!(at(&l, 8), vec![("Glob.LIMIT", "read", "const", KnowsConstant)]);
    // A constant holding a `Dict` or a `Ref`: the binding is content, what
    // the object holds is state.
    assert_eq!(at(&l, 41), vec![("Glob.CACHE", "read", "const", ApproxMutableObject)]);
    assert_eq!(at(&l, 43), vec![("Glob.TICKS", "read", "const", ApproxMutableObject)]);
    // Mutable globals: only the spelling, in a body, in a closure, assigned
    // twice, declared by `global later`, assigned only by a function, in a
    // submodule by a qualified name.
    for line in [12, 16, 37, 48, 52, 59, 87] {
        assert_eq!(at(&l, line).len(), 1, "line {line}");
        assert_eq!(at(&l, line)[0].3, KnowsState, "line {line}");
    }
    // WRONG: a content reference to a typed global's definition, read
    // directly or in a closure, while `set_level` assigns it.
    assert_eq!(at(&l, 25), vec![("Glob.level", "read", "global", WrongContentOfMutable)]);
    assert_eq!(at(&l, 27), vec![("Glob.level", "read", "global", WrongContentOfMutable)]);
    // WRONG: a typed global assigned twice is a "function with two methods",
    // so a use is a generic use.
    assert_eq!(at(&l, 32), vec![("Glob.mode2", "read", "global", WrongGenericOfMutable)]);
    // Left open: a constant defined twice (Julia ≥ 1.12 allows it: a new
    // world), a constant by a qualified name.
    assert_eq!(at(&l, 64), vec![("Glob.STEP", "read", "const", OpenConstant)]);
    assert_eq!(at(&l, 88), vec![("Glob.Inner.WIDTH", "read", "const", OpenConstant)]);
    // Left open: `global x = v` (or `global x` then `x = v`) in a body is
    // refused, so the definition writing the global has no id; a write by
    // `setproperty!` on a module has an id, and no fact says it writes.
    for line in [17, 18, 26, 47, 51] {
        assert_eq!(at(&l, line).len(), 1, "line {line}");
        assert_eq!((at(&l, line)[0].1, at(&l, line)[0].3), ("write", OpenRefused), "line {line}");
    }
    assert_eq!(at(&l, 89), vec![("Glob.Inner.depth", "write", "global", OpenWrite)]);
    // A constant's initializer reading another constant: no method, Julia
    // evaluated it at load; its binding kind is what is compared.
    assert_eq!(at(&l, 76), vec![("Glob.LIMIT", "read", "const", KnowsConstant)]);
    // Left open: a constant `@enum` made.
    assert_eq!(at(&l, 80), vec![("Glob.RED", "read", "const", OpenConstant)]);
    // A local named like a global (a parameter, an assignment in a
    // function): Julia's slot, Sophia's local.
    assert_eq!(at(&l, 55), vec![("Glob.scale", "local", "global", KnowsLocal)]);
    assert_eq!(at(&l, 56), vec![("Glob.scale", "local", "global", KnowsLocal)]);
    assert_eq!(at(&l, 92), vec![("Glob.LIMIT", "local", "const", KnowsLocal)]);
    assert_eq!(at(&l, 93), vec![("Glob.LIMIT", "local", "const", KnowsLocal)]);
    // WRONG: the argument of a callable object's method `(UNIT::Scale)(x)`,
    // named like the constant `UNIT`, is read as the constant (a content
    // reference), so the method's id depends on `const UNIT = Scale(1)`.
    // Only this form: a parameter or a local of a named function spelled
    // like a constant is a local (lines 92 and 93).
    assert_eq!(at(&l, 72), vec![("Glob.UNIT", "local", "const", WrongLocalAsGlobal)]);
    assert!(l.rows.iter().all(|r| r.outcome != NoJuliaUse), "{:?}", l.rows);
    // Review B-078 (Codex), 2 to 4: definitions sharing a line are not
    // compared, and their content references are not reported as
    // `NoJuliaUse`; a generic use on another definition's last line is
    // not that definition's (`gen_f`'s `mode2` is its argument).
    let mut amb: Vec<(usize, &str)> = l.ambiguous.iter().map(|a| (a.1, a.2.as_str())).collect();
    amb.sort_unstable();
    assert_eq!(amb, vec![(96, "Glob.LIMIT"), (98, "Glob.LIMIT"), (104, "Glob.mode2")]);
    assert!(at(&l, 96).is_empty() && at(&l, 98).is_empty() && at(&l, 104).is_empty());
    assert_eq!(at(&l, 102), vec![("Glob.mode2", "local", "global", KnowsLocal)]);
    // 5: a definition the normaliser refuses (`set_counter` is refused, so
    // initializers calling it are too) has no id and claims nothing.
    assert_eq!(binding(&l, "Glob.BAD"), ("const", vec![], OpenConstant));
    assert_eq!(binding(&l, "Glob.badlevel"), ("global", vec![], KnowsState));
    // 8: a mutable `AbstractString` held by a constant is a mutable object.
    assert_eq!(at(&l, 113), vec![("Glob.TEXT", "read", "const", ApproxMutableObject)]);
    assert_eq!(l.rows.len(), 30);
}

/// WRONG, the same rule seen in the store: a typed global is filed as a
/// generic function with methods, `jl.Glob.level` with one and
/// `jl.Glob.mode2` with two, while a constant is a named definition.
#[test]
fn typed_globals_are_stored_as_functions() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("g.sqlite")).unwrap());
    let (_, root) = sophia_ingest::module::package_root(Path::new(DIR)).unwrap();
    sophia_ingest::store_module(&s, "Glob", &root).unwrap();
    let methods = |p: &str| {
        let g = s.resolve(p).unwrap().unwrap_or_else(|| panic!("no {p}")).target.digest();
        s.methods(&g).unwrap().len()
    };
    assert_eq!(methods("jl.Glob.level"), 1);
    assert_eq!(methods("jl.Glob.mode2"), 2);
    assert_eq!(methods("jl.Glob.LIMIT"), 0);
    assert!(s.resolve("jl.Glob.scale").unwrap().is_none());
}

/// Replaces the kind column of every truth line about binding `b`.
fn kind(truth: &str, b: &str, k: &str) -> String {
    truth
        .lines()
        .map(|l| {
            let mut c: Vec<&str> = l.split('\t').collect();
            match c[0] {
                "binding" if c[1] == b => c[2] = k,
                "use" if c[4] == b => c[6] = k,
                _ => {}
            }
            c.join("\t") + "\n"
        })
        .collect()
}

/// The comparison reports each kind of wrong when Julia's side says so:
/// the fixture's truth with one binding's kind falsified.
#[test]
fn the_comparison_reports_wrongs() {
    use GlobalOutcome::*;
    let t = truth();
    // `LIMIT` mutable in Julia: Sophia's content reference is wrong.
    let l = ledger(&kind(&t, "Glob.LIMIT", "global"));
    assert_eq!(at(&l, 8), vec![("Glob.LIMIT", "read", "global", WrongContentOfMutable)]);
    assert_eq!(binding(&l, "Glob.LIMIT").2, WrongContentOfMutable);
    // `scale` a constant in Julia: Sophia's spelling leaves it open.
    let l = ledger(&kind(&t, "Glob.scale", "const"));
    assert_eq!(at(&l, 12), vec![("Glob.scale", "read", "const", OpenConstant)]);
    // A content reference Julia does not see.
    let without: String = t.lines().filter(|l| !l.starts_with("use\tGlob.jl\t8\t")).map(|l| l.to_string() + "\n").collect();
    let l = ledger(&without);
    assert_eq!(at(&l, 8), vec![("Glob.LIMIT", "read", "const", NoJuliaUse)]);
    // A truncated line is counted, not compared.
    let l = ledger(&(t.clone() + "use\tGlob.jl\t8\n"));
    assert_eq!(l.malformed, 1);
}

/// Review B-078 (Codex), 6: a truth line with an unknown access, a line
/// number that is no number, a kind that contradicts the binding's, or a
/// binding listed with two kinds is counted as malformed and not compared.
#[test]
fn malformed_truth_is_not_compared() {
    let t = truth();
    let line12 = t.lines().find(|l| l.starts_with("use\tGlob.jl\t12\t")).unwrap();
    for bad in [line12.replace("\tread\t", "\traed\t"), line12.replace("\t12\t", "\toops\t"), line12.replace("\tglobal", "\tconst")] {
        let falsified: String = t.lines().map(|l| if l == line12 { bad.clone() } else { l.to_string() }).map(|l| l + "\n").collect();
        let l = ledger(&falsified);
        assert_eq!(l.malformed, 1, "{bad}");
        assert!(at(&l, 12).is_empty() && l.outside.is_empty(), "{bad}");
    }
    let l = ledger(&(t.clone() + "binding\tGlob.scale\tconst\t-\timmutable\n"));
    assert_eq!(l.malformed, 5, "the binding twice, and its four uses (lines 12, 55, 56, 59)");
    assert!(l.bindings.iter().all(|b| b.binding != "Glob.scale"));
}
