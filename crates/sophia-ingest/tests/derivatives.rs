//! Experiment E-5 (rules vs derived derivatives) on the fixture
//! `julia-semantics/rules`: Zygote's derived programs for a package in eight
//! variants (`rules/truth.tsv`, printed by `julia-semantics/derivatives.jl`;
//! see `derivatives.md` to regenerate), against three keys computed without
//! running Zygote. Each expected result is the one [[Semantic Boundary
//! Experiments]] §E-5 records.

use std::path::{Path, PathBuf};

use sophia_ingest::derivative::{DerivativeLedger, Key, Oracle, derivative_ledger};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/julia-semantics/rules");
const VARIANTS: [&str; 10] = [
    "base",
    "rule_g",
    "adjoint_g",
    "adjoint_g_other",
    "nondiff_sq",
    "method_g32",
    "edit_unused",
    "pirate_g",
    "pirate_g_other",
    "pirate_sin",
];

fn ledger() -> (Oracle, DerivativeLedger) {
    let dir = Path::new(DIR);
    let mut sources: Vec<(&str, PathBuf)> = VARIANTS.iter().map(|v| (*v, dir.join("variants").join(v))).collect();
    // The session added `rule_g`'s file to `base`: its source is `rule_g`'s.
    sources.push(("base+session", dir.join("variants/rule_g")));
    let tsv = std::fs::read_to_string(dir.join("truth.tsv")).unwrap();
    derivative_ledger("Rules", &sources, &tsv).unwrap()
}

fn both(fs: &[&str]) -> Vec<(String, String)> {
    let mut v = Vec::new();
    for t in ["Float64", "Float32"] {
        for f in fs {
            v.push((f.to_string(), t.to_string()));
        }
    }
    v
}

/// Every function with a complete program (`dyn`'s is not).
fn all() -> Vec<(String, String)> {
    both(&["f1", "f2", "f3", "f4", "g", "sq", "h", "p", "q"])
}

#[test]
fn every_gradient_is_parsed() {
    let (o, l) = ledger();
    // Eleven labels (ten variants and the session), ten functions, two types.
    assert_eq!(o.derived.len(), 11 * 20);
    assert_eq!(l.rows.len(), 11 * 20);
    assert!(o.derived.iter().all(|d| !d.pieces.is_empty()));
    // Every piece's text was printed once.
    assert!(o.derived.iter().flat_map(|d| &d.pieces).all(|(_, _, d)| o.texts.contains_key(d)));
    assert!(o.versions["base"].contains(&("Zygote".to_string(), "0.7.18".to_string())));
    assert!(o.inconsistent.is_empty());
}

#[test]
fn which_derived_programs_change() {
    let (_, l) = ledger();
    // A rule for `g`, in the package, in Zygote's system or outside the
    // package: the derivatives of `g` and its callers change, the others not.
    for v in ["rule_g", "adjoint_g", "adjoint_g_other", "pirate_g", "pirate_g_other"] {
        assert_eq!(l.changed("base", v), both(&["f1", "f2", "g"]), "{v}");
    }
    // Two rules for `g` with different pullback bodies give different
    // programs (Codex's review, findings 1 and 2: the body of a rule's
    // closure, and the `ZygoteRules.adjoint` method `@adjoint` makes).
    for (a, b) in [("pirate_g", "pirate_g_other"), ("adjoint_g", "adjoint_g_other")] {
        assert_eq!(l.changed(a, b), both(&["f1", "f2", "g"]), "{a} {b}");
    }
    assert_eq!(l.changed("base", "nondiff_sq"), both(&["f1", "f2", "f3", "sq"]));
    // A rule for `sin` at Float64 outside the package: only at Float64.
    assert_eq!(
        l.changed("base", "pirate_sin"),
        vec![("f1".into(), "Float64".into()), ("f2".into(), "Float64".into()), ("g".into(), "Float64".into())]
    );
    // A method of `g` for Float32: only the Float32 derivatives.
    assert_eq!(
        l.changed("base", "method_g32"),
        vec![("f1".into(), "Float32".into()), ("f2".into(), "Float32".into()), ("g".into(), "Float32".into())]
    );
    assert!(l.changed("base", "edit_unused").is_empty());
}

#[test]
fn equal_programs_compute_equal_gradients() {
    let (_, l) = ledger();
    for (i, a) in l.rows.iter().enumerate() {
        for b in &l.rows[i + 1..] {
            // Any two gradients, of one function or of two (`p` and `q`
            // reach the same instances from different entry points).
            if a.complete && b.complete && a.output == b.output {
                assert_eq!(a.gradient, b.gradient, "{} {} {} {}", a.label, b.label, a.target, a.argtype);
            }
        }
    }
    // `@non_differentiable sq(x)`: the gradient of `sq` is `nothing`.
    assert_eq!(l.row("nondiff_sq", "sq", "Float64").unwrap().gradient, "nothing");
}

#[test]
fn the_store_key_misses_rules_made_by_macros_and_rules_from_outside() {
    let (_, l) = ledger();
    assert!(l.rekeyed(Key::Store, "base", "edit_unused").is_empty());
    assert_eq!(l.rekeyed(Key::Store, "base", "rule_g"), both(&["f1", "f2", "g"]));
    // Unsound: the program changed, the key did not.
    assert!(l.rekeyed(Key::Store, "base", "nondiff_sq").is_empty());
    assert!(l.rekeyed(Key::Store, "base", "pirate_g").is_empty());
    assert!(l.rekeyed(Key::Store, "base", "pirate_sin").is_empty());
    // Right by accident: Sophia takes `Zygote.@adjoint g(x) = …` for a
    // second method of `g` (E-1 part 5: a definition in a macro call has the
    // id it has without the macro), so `g` becomes generic and the ids move.
    assert_eq!(l.rekeyed(Key::Store, "base", "adjoint_g"), both(&["f1", "f2", "g"]));
    assert_eq!(l.row("base", "g", "Float64").unwrap().definitions, 1);
    assert_eq!(l.row("adjoint_g", "g", "Float64").unwrap().definitions, 2);
    // Per function, not per signature: `g(::Float32)` re-keys Float64 too.
    assert_eq!(l.rekeyed(Key::Store, "base", "method_g32"), both(&["f1", "f2", "g"]));
    // The package's own rule for `h` is in the key of `h` and `f4` only.
    assert_eq!(l.row("base", "f4", "Float64").unwrap().package_rules, 1);
    assert_eq!(l.row("base", "f1", "Float64").unwrap().package_rules, 0);
    let s = l.score(Key::Store);
    // Complete programs only: 198 rows, 19 503 pairs.
    assert_eq!((s.pairs, s.unsound.len(), s.missed), (19503, 113, 7));
}

#[test]
fn rule_facts_from_the_method_table_leave_only_rules_for_functions_outside_the_package() {
    let (_, l) = ledger();
    assert_eq!(l.rekeyed(Key::Rules, "base", "nondiff_sq"), both(&["f1", "f2", "f3", "sq"]));
    assert_eq!(l.rekeyed(Key::Rules, "base", "pirate_g"), both(&["f1", "f2", "g"]));
    assert_eq!(l.rekeyed(Key::Rules, "pirate_g", "pirate_g_other"), both(&["f1", "f2", "g"]));
    assert!(l.rekeyed(Key::Rules, "base", "pirate_sin").is_empty());
    let s = l.score(Key::Rules);
    assert_eq!((s.unsound.len(), s.missed), (7, 7));
    assert!(s.unsound.iter().all(|(a, b, _, t)| (a == "pirate_sin" || b == "pirate_sin") && t == "Float64"), "{:?}", s.unsound);
}

#[test]
fn whole_rule_tables_are_sound_and_coarse() {
    let (_, l) = ledger();
    let s = l.score(Key::Tables);
    assert_eq!((s.unsound.len(), s.missed), (0, 590));
    // Any rule anywhere re-keys every derivative.
    assert_eq!(l.rekeyed(Key::Tables, "base", "pirate_sin"), all());
    assert!(l.rekeyed(Key::Tables, "base", "edit_unused").is_empty());
}

#[test]
fn a_session_that_adds_a_rule_generates_the_new_program_and_runs_the_old_one() {
    let (o, l) = ledger();
    // Zygote's generators, called in the session after the rule was added,
    // give exactly the program of a fresh process with the rule.
    for r in l.rows.iter().filter(|r| r.label == "base+session") {
        let fresh = l.row("rule_g", &r.target, &r.argtype).unwrap();
        assert_eq!(r.output, fresh.output, "{} {}", r.target, r.argtype);
        assert_eq!(r.keys, fresh.keys);
    }
    // But the gradients the session runs never call the rule (a fresh
    // process calls it 8 times: f1, f2, g, dyn at two types), not even from
    // a caller compiled after the rule was added.
    let calls = |label: &str, what: &str| o.calls.iter().find(|c| c.0 == label && c.1 == what).map(|c| c.2);
    assert_eq!(calls("rule_g", "rows"), Some(8));
    assert_eq!(calls("base+session", "rows"), Some(0));
    assert_eq!(calls("base+session", "g-again"), Some(0));
    assert_eq!(calls("base+session", "fresh-caller"), Some(0));
}

#[test]
fn transformation_is_a_small_part_of_the_first_call() {
    let (o, _) = ledger();
    // Zygote's own calls for every reached instance, its decomposition
    // cache emptied first (finding 6), against the first gradient call, over
    // the complete programs of the fresh variants (the session rows are
    // second calls). Timings vary per run; the bound is loose (3.3 % here).
    let fresh = || o.derived.iter().filter(|d| !d.label.contains('+') && d.complete);
    let (generate, first): (u64, u64) = fresh().fold((0, 0), |(g, f), d| (g + d.ns_gen, f + d.ns_first));
    assert!(generate * 10 < first, "generators {generate} ns, first calls {first} ns");
    // On the filled cache they take less.
    let cached: u64 = fresh().map(|d| d.ns_cached).sum();
    assert!(cached < generate, "cached {cached} ns, cold {generate} ns");
}

#[test]
fn no_key_is_shared_by_two_functions() {
    // `p` and `q` call each other: one closure, two derivatives. A key that
    // names the closure but not the root gives them one id (Codex's review,
    // finding 3).
    let (_, l) = ledger();
    for k in [Key::Store, Key::Rules, Key::Tables] {
        for (i, a) in l.rows.iter().enumerate() {
            for b in &l.rows[i + 1..] {
                if a.keys[&k] == b.keys[&k] {
                    assert_eq!(a.target, b.target, "{k:?}: {} {} and {} {}", a.label, a.target, b.label, b.target);
                }
            }
        }
    }
}

#[test]
fn a_walk_through_runtime_dispatch_is_incomplete() {
    // `dyn` calls `g` through a value Julia cannot infer: the walk cannot
    // name what runs, so the program is not complete (finding 5).
    let (o, _) = ledger();
    for d in o.derived.iter().filter(|d| d.target == "dyn") {
        assert!(!d.complete, "{} {}", d.label, d.argtype);
    }
    assert!(o.derived.iter().filter(|d| d.target != "dyn").all(|d| d.complete));
}

#[test]
fn versions_belong_to_their_run() {
    // Two runs of one variant with different Zygote versions: different
    // keys, even when their programs print alike (finding 7).
    let dir = Path::new(DIR).join("variants/base");
    let run = |label: &str, zygote: &str| {
        format!(
            "julia\t1.12.7\npackage\tZygote\t{zygote}\nderived\t{label}\tg\tFloat64\t1\tall\t1\t1\t1\t1\t1.0\ninstance\t{label}\tg\tFloat64\tTuple{{typeof(Main.Rules.g), Float64}}\tir\tabc\n"
        )
    };
    let tsv = run("a", "0.7.18") + &run("b", "0.7.19");
    let (_, l) = derivative_ledger("Rules", &[("a", dir.clone()), ("b", dir)], &tsv).unwrap();
    let (a, b) = (l.row("a", "g", "Float64").unwrap(), l.row("b", "g", "Float64").unwrap());
    assert_eq!(a.output, b.output);
    for k in [Key::Store, Key::Rules, Key::Tables] {
        assert_ne!(a.keys[&k], b.keys[&k], "{k:?}");
    }
}
