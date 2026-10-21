//! Round trip (SPEC-005 §5, rule 2): printing a definition in an editable
//! profile and parsing the text back gives the same content id, and printing
//! again gives the same text.

use std::collections::HashMap;

use proptest::prelude::*;
use sophia_hash::{Digest, Record};
use sophialang_core::samples::{program, sample_nominal};
use sophialang_core::*;
use sophialang_syntax::*;

/// A store-like environment built from the sample program.
struct SampleEnv {
    names: HashMap<String, Found>,
    records: HashMap<Digest, Record>,
}

impl SampleEnv {
    fn new() -> (Self, Vec<(&'static str, Record)>) {
        let (names, defs): (Vec<_>, Vec<Definition>) = program().into_iter().unzip();
        let records = normalise_batch(&defs, &mut Default::default()).unwrap();
        let mut env = SampleEnv { names: HashMap::new(), records: HashMap::new() };
        for (n, r) in names.iter().zip(&records) {
            env.names.insert(n.to_string(), Found::Content(r.id));
            env.records.insert(r.id, r.clone());
        }
        for (n, _, id) in samples::nominals() {
            env.names.insert(n.to_string(), Found::Nominal(id));
        }
        (env, names.into_iter().zip(records).collect())
    }
}

impl Env for SampleEnv {
    fn lookup(&self, path: &str) -> Option<Found> {
        self.names.get(path).copied()
    }
    fn constructors(&self, data: &Digest) -> Option<Vec<String>> {
        let d = definition_of(self.records.get(data)?).ok()?;
        match d.body {
            Body::Constructors(cs) => Some(cs.into_iter().map(|c| c.name).collect()),
            _ => None,
        }
    }
    fn names_of(&self, target: &Digest) -> Vec<String> {
        let mut v: Vec<String> = self
            .names
            .iter()
            .filter(|(_, f)| matches!(f, Found::Content(d) | Found::Nominal(d) if d == target))
            .map(|(n, _)| n.clone())
            .collect();
        v.sort();
        v
    }
}

fn short(path: &str) -> &str {
    path.rsplit('.').next().unwrap()
}

/// Prints the records (one recursive group, or one definition), parses the
/// text, normalises the result and checks ids and idempotence.
fn check(env: &SampleEnv, profile: &Profile, group: &[(&str, &Record)]) -> String {
    let defs: Vec<Definition> = group.iter().map(|(_, r)| definition_of(r).unwrap()).collect();
    let items: Vec<ToPrint> = group
        .iter()
        .zip(&defs)
        .map(|((n, r), d)| ToPrint { name: short(n), def: d, id: Some(r.id), index: r.index, relation: false })
        .collect();
    let text = print(env, "sample", profile, &items);
    let (parsed, _warnings) = parse_text(env, "sample", &text, Limits::default())
        .unwrap_or_else(|e| panic!("{} did not parse:\n{text}\n{e:?}", profile.name));
    let parsed_defs: Vec<Definition> = parsed.iter().map(|i| i.definition().clone()).collect();
    let rehashed = normalise_batch(&parsed_defs, &mut Default::default()).unwrap();
    for ((n, r), h) in group.iter().zip(&rehashed) {
        assert_eq!(h.id, r.id, "{} changed under a round trip in {}:\n{text}", n, profile.name);
    }
    // Printing the parsed result again gives the same text.
    let again: Vec<ToPrint> = group
        .iter()
        .zip(&parsed_defs)
        .zip(&rehashed)
        .map(|(((n, _), d), h)| ToPrint { name: short(n), def: d, id: Some(h.id), index: h.index, relation: false })
        .collect();
    assert_eq!(print(env, "sample", profile, &again), text, "printing is not idempotent");
    text
}

#[test]
fn the_sample_program_round_trips_in_julia_and_rust() {
    let (env, named) = SampleEnv::new();
    for profile in [Profile::julia(), Profile::built_in("rust").unwrap()] {
        let mut done = vec![false; named.len()];
        for i in 0..named.len() {
            if done[i] {
                continue;
            }
            let comp = named[i].1.component;
            let mut group: Vec<(&str, &Record)> = named
                .iter()
                .enumerate()
                .filter(|(_, (_, r))| r.component == comp && r.id != named[6].1.id || (r.component == comp && r.id == named[i].1.id))
                .map(|(k, (n, r))| {
                    done[k] = true;
                    (*n, r)
                })
                .collect();
            group.dedup_by_key(|(_, r)| r.id);
            group.sort_by_key(|(_, r)| r.index);
            check(&env, &profile, &group);
        }
    }
}

#[test]
fn the_julia_text_reads_like_julia() {
    let (env, named) = SampleEnv::new();
    let add = named.iter().find(|(n, _)| *n == "sample.add").unwrap();
    let text = check(&env, &Profile::julia(), &[(add.0, &add.1)]);
    assert_eq!(
        text,
        "function add(a::Nat, b::Nat)::Nat\n    match a :: Nat\n        zero => b\n        succ(k) => Nat.succ(add(k, b))\n    end\nend\n"
    );
    let rust = check(&env, &Profile::built_in("rust").unwrap(), &[(add.0, &add.1)]);
    assert!(rust.starts_with("fn add(a: Nat, b: Nat) -> Nat {\n"), "{rust}");

    let even: Vec<(&str, &Record)> =
        named.iter().filter(|(n, _)| *n == "sample.even" || *n == "sample.odd").map(|(n, r)| (*n, r)).collect();
    let mut even = even;
    even.sort_by_key(|(_, r)| r.index);
    let text = check(&env, &Profile::julia(), &even);
    assert!(text.contains("odd(k)") && text.contains("even(k)"), "{text}");

    let rule = named.iter().find(|(n, _)| *n == "sample.ancestor_step").unwrap();
    let text = check(&env, &Profile::julia(), &[(rule.0, &rule.1)]);
    assert_eq!(text, "rule ancestor_step(X, Y, Z) @weight(0.9) = ancestor(X, Z) :- parent(X, Y), ancestor(Y, Z)\n");

    let double = named.iter().find(|(n, _)| *n == "sample.double").unwrap();
    assert_eq!(check(&env, &Profile::julia(), &[(double.0, &double.1)]), "function double(x::Int64)::Int64\n    x + x\nend\n");
}

#[test]
fn relations_print_from_their_signature() {
    let (env, named) = SampleEnv::new();
    let sig = definition_of(&named[6].1).unwrap();
    let text = print(
        &env,
        "sample",
        &Profile::julia(),
        &[ToPrint { name: "parent", def: &sig, id: None, index: 0, relation: true }],
    );
    assert_eq!(text, "relation parent::Nat --> Nat --> Prop\n");
    let (items, _) = parse_text(&env, "sample", &text, Limits::default()).unwrap();
    assert!(matches!(&items[0], Item::Relation { name, signature } if name == "parent" && *signature == sig));
    assert_ne!(sample_nominal("sample.parent"), sample_nominal("sample.ancestor"));
}

#[test]
fn errors_and_warnings_have_ranges() {
    let env = EmptyEnv;
    let e = parse_text(&env, "", "function f(x::Int64)::Int64\n    x +\nend\n", Limits::default()).unwrap_err();
    assert_eq!(e.len(), 1);
    assert!(e[0].span.start > 0, "{:?}", e[0]);
    let (items, warnings) = parse_text(&env, "", "const c::Int64 = math.sqrt(2)\n", Limits::default()).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("math.sqrt"));
    let long = "const c::Int64 = ".to_string() + &"(".repeat(300) + "1" + &")".repeat(300);
    assert!(parse_text(&env, "", &long, Limits::default()).is_err(), "depth is bounded");
    let big = "x".repeat(600 * 1024);
    assert!(parse_text(&env, "", &big, Limits::default()).is_err(), "size is bounded");
}

// ---- random terms ----

fn arb_term() -> impl Strategy<Value = Term> {
    let leaf = prop_oneof![
        (0u32..3).prop_map(Term::Var),
        any::<[u8; 32]>().prop_map(|b| Term::Ref(Digest(b))),
        any::<[u8; 32]>().prop_map(|b| Term::Nominal(Digest(b))),
        Just(Term::Nominal(samples::builtin("Int64"))),
        (0u32..3).prop_map(Term::Type),
        Just(Term::Prop),
        any::<i64>().prop_map(|v| Term::Lit(Lit::Int64(v))),
        any::<u64>().prop_map(|v| Term::Lit(Lit::UInt64(v))),
        any::<f64>().prop_map(|v| Term::Lit(Lit::float(v))),
        any::<String>().prop_map(|s| Term::Lit(Lit::String(s))),
        any::<char>().prop_map(|c| Term::Lit(Lit::Char(c))),
        any::<bool>().prop_map(|b| Term::Lit(Lit::Bool(b))),
        "[a-z][a-z_]{0,6}".prop_map(Term::Prim),
        prop::collection::vec("u_[a-z]{1,4}", 1..3).prop_map(Term::Unresolved),
    ];
    let binder = ("[a-z]{0,3}", prop_oneof![Just(Usage::Zero), Just(Usage::One), Just(Usage::Many)], any::<bool>())
        .prop_map(|(name, usage, implicit)| Binder { name, usage: if implicit && usage == Usage::Many { Usage::Zero } else { usage }, implicit });
    leaf.prop_recursive(5, 40, 4, move |inner| {
        let ops = prop_oneof![Just("+"), Just("-"), Just("*"), Just("/"), Just("=="), Just("<="), Just("!=")];
        prop_oneof![
            (binder.clone(), prop::option::of(inner.clone()), inner.clone())
                .prop_map(|(binder, domain, body)| Term::Lam { binder, domain: domain.map(Box::new), body: Box::new(body) }),
            (inner.clone(), prop::collection::vec(inner.clone(), 0..3)).prop_map(|(h, a)| Term::App(Box::new(h), a)),
            (ops, inner.clone(), inner.clone())
                .prop_map(|(o, l, r)| Term::App(Box::new(Term::Nominal(samples::builtin(o))), vec![l, r])),
            (binder.clone(), inner.clone(), inner.clone())
                .prop_map(|(binder, d, c)| Term::Pi { binder, domain: Box::new(d), codomain: Box::new(c) }),
            (binder.clone(), prop::option::of(inner.clone()), inner.clone(), inner.clone()).prop_map(|(binder, ty, v, b)| {
                Term::Let { binder, ty: ty.map(Box::new), value: Box::new(v), body: Box::new(b) }
            }),
            (
                inner.clone(),
                prop::option::of(inner.clone()),
                prop::collection::vec((0u32..3, prop::collection::vec("[a-z]{1,2}", 0..2), inner.clone()), 0..3)
            )
                .prop_map(|(s, data, cs)| Term::Match {
                    scrutinee: Box::new(s),
                    data: data.map(Box::new),
                    motive: None,
                    cases: cs
                        .into_iter()
                        .map(|(constructor, bs, body)| Case {
                            constructor,
                            binders: bs.into_iter().map(Binder::named).collect(),
                            body
                        })
                        .collect()
                }),
            (inner.clone(), 0u32..3).prop_map(|(t, index)| Term::Con { ty: Box::new(t), index }),
            inner.clone().prop_map(|t| Term::Hole(Box::new(t))),
            inner.prop_map(|t| Term::Unsafe(Aspect::Memory, Box::new(t))),
        ]
    })
}

/// Replaces variables that would be free with a literal, so terms are closed.
fn close(t: Term, depth: u32) -> Term {
    let under = |t: Term, k: u32| close(t, depth + k);
    match t {
        Term::Var(i) if i >= depth => Term::Lit(Lit::Int64(i as i64)),
        Term::Lam { binder, domain, body } => {
            Term::Lam { binder, domain: domain.map(|d| Box::new(close(*d, depth))), body: Box::new(under(*body, 1)) }
        }
        Term::Pi { binder, domain, codomain } => {
            Term::Pi { binder, domain: Box::new(close(*domain, depth)), codomain: Box::new(under(*codomain, 1)) }
        }
        Term::Let { binder, ty, value, body } => Term::Let {
            binder,
            ty: ty.map(|t| Box::new(close(*t, depth))),
            value: Box::new(close(*value, depth)),
            body: Box::new(under(*body, 1)),
        },
        Term::Match { scrutinee, data, motive, cases } => Term::Match {
            scrutinee: Box::new(close(*scrutinee, depth)),
            data: data.map(|d| Box::new(close(*d, depth))),
            motive,
            cases: cases
                .into_iter()
                .map(|c| {
                    let k = c.binders.len() as u32;
                    Case { body: under(c.body, k), ..c }
                })
                .collect(),
        },
        Term::App(h, args) => Term::App(Box::new(close(*h, depth)), args.into_iter().map(|a| close(a, depth)).collect()),
        Term::Con { ty, index } => Term::Con { ty: Box::new(close(*ty, depth)), index },
        Term::Hole(t) => Term::Hole(Box::new(close(*t, depth))),
        Term::Unsafe(a, t) => Term::Unsafe(a, Box::new(close(*t, depth))),
        t => t,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(400)))]
    #[test]
    fn random_definitions_round_trip(body in arb_term(), ty in arb_term(), rust in any::<bool>()) {
        let profile = if rust { Profile::built_in("rust").unwrap() } else { Profile::julia() };
        for def in [Definition::test(close(body.clone(), 0)), Definition::method(close(ty, 0), close(body, 0))] {
            let id = normalise_one(&def).unwrap().id;
            let items = [ToPrint { name: "t", def: &def, id: Some(id), index: 0, relation: false }];
            let text = print(&EmptyEnv, "", &profile, &items);
            // Rust's blocks are braces all the way down: `let` and `match` too.
            prop_assert!(!rust || !text.lines().any(|l| l.trim() == "end" || l.trim_start().starts_with("end)")), "{}", text);
            let parsed = parse_text(&EmptyEnv, "", &text, Limits::default());
            prop_assert!(parsed.is_ok(), "did not parse:\n{}\n{:?}", text, parsed.err());
            let (items2, _) = parsed.unwrap();
            let def2 = items2[0].definition().clone();
            let id2 = normalise_one(&def2).unwrap().id;
            prop_assert_eq!(id, id2, "changed:\n{}\n{:?}\n{:?}", text, def, def2);
            let again = print(&EmptyEnv, "", &profile, &[ToPrint { name: "t", def: &def2, id: Some(id2), index: 0, relation: false }]);
            prop_assert_eq!(again, text);
        }
    }
}
