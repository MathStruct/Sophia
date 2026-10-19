//! Identity properties of sophialang's normaliser (SPEC-001 §4, SPEC-003 §3;
//! Milestones M0).

use proptest::prelude::*;
use sophia_hash::component::Budget;
use sophia_hash::{Digest, Record, Status};
use sophialang_core::encode::{decode, encode_resolved, for_each_name};
use sophialang_core::samples::{builtin, program};
use sophialang_core::*;

fn batch(defs: &[Definition]) -> Vec<Record> {
    normalise_batch(defs, &mut Budget::default()).unwrap()
}

fn sample_records() -> Vec<Record> {
    let defs: Vec<Definition> = program().into_iter().map(|(_, d)| d).collect();
    batch(&defs)
}

fn rename_all(mut d: Definition, prefix: &str) -> Definition {
    let mut i = 0;
    for_each_name(&mut d, &mut |n| {
        *n = format!("{prefix}{i}");
        i += 1;
    });
    d
}

#[test]
fn normalising_is_deterministic() {
    assert_eq!(sample_records(), sample_records());
}

#[test]
fn names_do_not_change_ids() {
    let defs: Vec<Definition> = program().into_iter().map(|(_, d)| d).collect();
    let renamed: Vec<Definition> = defs.iter().cloned().map(|d| rename_all(d, "zz")).collect();
    let a = batch(&defs);
    let b = batch(&renamed);
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.id, y.id);
        if !x.display.is_empty() {
            assert_ne!(x.display, y.display, "display data carries the names");
        }
    }
}

#[test]
fn every_id_is_distinct_except_by_design() {
    let r = sample_records();
    let mut ids: Vec<Digest> = r.iter().map(|r| r.id).collect();
    ids.sort();
    ids.dedup();
    // parent and ancestor are both `Nat → Nat → Prop` with no body: one definition.
    assert_eq!(ids.len(), r.len() - 1);
    assert_eq!(r[6].id, r[7].id);
}

#[test]
fn mutual_recursion_shares_a_component_and_is_order_independent() {
    let r = sample_records();
    let (even, odd) = (&r[2], &r[3]);
    assert_eq!(even.component, odd.component);
    assert_eq!(even.component_size, 2);
    assert_ne!(even.id, odd.id);
    assert!(Record::verify(&{
        let mut m = vec![even, odd];
        m.sort_by_key(|r| r.index);
        m
    }));

    // The same program with even and odd swapped gives the same ids.
    let mut defs: Vec<Definition> = program().into_iter().map(|(_, d)| d).collect();
    defs.swap(2, 3);
    let swap = |t: Term| match t {
        Term::Draft(2) => Term::Draft(3),
        Term::Draft(3) => Term::Draft(2),
        t => t,
    };
    let defs: Vec<Definition> = defs.into_iter().map(|d| d.map_terms(&mut { swap })).collect();
    let s = batch(&defs);
    assert_eq!(s[2].id, odd.id);
    assert_eq!(s[3].id, even.id);
}

#[test]
fn a_dependency_change_changes_the_dependant() {
    let mut defs: Vec<Definition> = program().into_iter().map(|(_, d)| d).collect();
    let before = batch(&defs);
    if let Body::Constructors(cs) = &mut defs[0].body {
        cs.push(Constructor { name: "omega".into(), ty: Term::Draft(0) });
    }
    let after = batch(&defs);
    assert_ne!(before[0].id, after[0].id, "Nat changed");
    assert_ne!(before[1].id, after[1].id, "add depends on Nat");
    assert_ne!(before[10].id, after[10].id, "the test depends on add");
    assert_eq!(before[11].id, after[11].id, "double does not depend on Nat");
}

#[test]
fn holes_and_unresolved_names_are_incomplete_and_hashed() {
    let r = sample_records();
    assert_eq!(r[4].status, Status::Incomplete);
    assert_eq!(r[5].status, Status::Incomplete);
    assert_eq!(r[1].status, Status::Complete);

    let int = || Term::Nominal(builtin("Int64"));
    let h1 = Definition::method(Term::arrow(int(), int()), Term::lam("x", Term::hole(int())));
    let h2 = Definition::method(Term::arrow(int(), int()), Term::lam("x", Term::hole(Term::Nominal(builtin("UInt64")))));
    assert_ne!(normalise_one(&h1).unwrap().id, normalise_one(&h2).unwrap().id);

    let u1 = Definition::method(Term::arrow(int(), int()), Term::Unresolved(vec!["a".into()]));
    let u2 = Definition::method(Term::arrow(int(), int()), Term::Unresolved(vec!["b".into()]));
    assert_ne!(normalise_one(&u1).unwrap().id, normalise_one(&u2).unwrap().id);
}

#[test]
fn usage_and_unsafe_marks_are_hashed() {
    let int = || Term::Nominal(builtin("Int64"));
    let body = || Term::lam("x", Term::Var(0));
    let many = Definition::method(Term::arrow(int(), int()), body());
    let mut one = many.clone();
    if let Some(Term::Pi { binder, .. }) = &mut one.ty {
        binder.usage = Usage::One;
    }
    let marked = Definition::method(Term::arrow(int(), int()), Term::Unsafe(Aspect::Memory, Box::new(body())));
    let a = normalise_one(&many).unwrap().id;
    assert_ne!(a, normalise_one(&one).unwrap().id);
    assert_ne!(a, normalise_one(&marked).unwrap().id);
}

#[test]
fn the_declared_type_is_hashed() {
    let id_int = Definition::method(
        Term::arrow(Term::Nominal(builtin("Int64")), Term::Nominal(builtin("Int64"))),
        Term::lam("x", Term::Var(0)),
    );
    let id_poly = Definition::method(
        Term::Pi {
            binder: Binder::implicit("A"),
            domain: Box::new(Term::Type(0)),
            codomain: Box::new(Term::arrow(Term::Var(0), Term::Var(1))),
        },
        Term::lam("x", Term::Var(0)),
    );
    assert_ne!(normalise_one(&id_int).unwrap().id, normalise_one(&id_poly).unwrap().id);
}

#[test]
fn stored_records_decode_and_rehash_to_the_same_id() {
    let defs: Vec<Definition> = program().into_iter().map(|(_, d)| d).collect();
    for r in sample_records() {
        let d = definition_of(&r).unwrap();
        if r.component_size == 1 {
            assert_eq!(normalise_one(&d).unwrap().id, r.id, "{}", r.kind);
        }
        // Names come back from the display data.
        let mut names = Vec::new();
        let mut d2 = d.clone();
        for_each_name(&mut d2, &mut |n| names.push(n.clone()));
        assert!(names.iter().all(|n| !n.is_empty()) || names.is_empty());
    }
    assert_eq!(defs.len(), 16);
}

#[test]
fn dangling_drafts_are_rejected() {
    let d = Definition::constant(Term::Type(0), Term::Draft(5));
    assert!(matches!(
        normalise_batch(&[d], &mut Budget::default()),
        Err(NormaliseError::DanglingDraft { from: 0, to: 5 })
    ));
}

// ---- properties over random terms ----

fn arb_binder() -> impl Strategy<Value = Binder> {
    ("[a-z]{1,3}", prop_oneof![Just(Usage::Zero), Just(Usage::One), Just(Usage::Many)], any::<bool>())
        .prop_map(|(name, usage, implicit)| Binder { name, usage, implicit })
}

fn arb_lit() -> impl Strategy<Value = Lit> {
    prop_oneof![
        any::<i64>().prop_map(Lit::Int64),
        any::<u64>().prop_map(Lit::UInt64),
        any::<u64>().prop_map(Lit::Float64),
        "[a-z ]{0,8}".prop_map(Lit::String),
        any::<bool>().prop_map(Lit::Bool),
        any::<char>().prop_map(Lit::Char),
    ]
}

fn arb_term() -> impl Strategy<Value = Term> {
    let leaf = prop_oneof![
        (0u32..4).prop_map(Term::Var),
        any::<[u8; 32]>().prop_map(|b| Term::Ref(Digest(b))),
        any::<[u8; 32]>().prop_map(|b| Term::Nominal(Digest(b))),
        (0u32..3).prop_map(Term::Type),
        Just(Term::Prop),
        arb_lit().prop_map(Term::Lit),
        "[a-z_]{1,6}".prop_map(Term::Prim),
        prop::collection::vec("[a-z]{1,4}", 1..3).prop_map(Term::Unresolved),
    ];
    leaf.prop_recursive(5, 48, 4, |inner| {
        prop_oneof![
            (arb_binder(), prop::option::of(inner.clone()), inner.clone())
                .prop_map(|(binder, dom, body)| Term::Lam { binder, domain: dom.map(Box::new), body: Box::new(body) }),
            (inner.clone(), prop::collection::vec(inner.clone(), 0..3))
                .prop_map(|(h, a)| Term::App(Box::new(h), a)),
            (arb_binder(), inner.clone(), inner.clone()).prop_map(|(binder, d, c)| Term::Pi {
                binder,
                domain: Box::new(d),
                codomain: Box::new(c)
            }),
            (arb_binder(), prop::option::of(inner.clone()), inner.clone(), inner.clone()).prop_map(
                |(binder, ty, v, b)| Term::Let { binder, ty: ty.map(Box::new), value: Box::new(v), body: Box::new(b) }
            ),
            (
                inner.clone(),
                prop::option::of(inner.clone()),
                prop::collection::vec((0u32..3, prop::collection::vec(arb_binder(), 0..2), inner.clone()), 0..3)
            )
                .prop_map(|(s, m, cs)| Term::Match {
                    scrutinee: Box::new(s),
                    motive: m.map(Box::new),
                    cases: cs.into_iter().map(|(constructor, binders, body)| Case { constructor, binders, body }).collect()
                }),
            (inner.clone(), 0u32..4).prop_map(|(t, index)| Term::Con { ty: Box::new(t), index }),
            inner.clone().prop_map(|t| Term::Hole(Box::new(t))),
            inner.prop_map(|t| Term::Unsafe(Aspect::Concurrency, Box::new(t))),
        ]
    })
}

fn strip_names(mut d: Definition) -> Definition {
    for_each_name(&mut d, &mut |n| n.clear());
    d
}

proptest! {
    #[test]
    fn encode_decode_round_trips(ty in prop::option::of(arb_term()), body in arb_term()) {
        let def = Definition { kind: Kind::Constant, ty, body: Body::Term(body) };
        let bytes = encode_resolved(&def);
        prop_assert_eq!(decode(&bytes).unwrap(), strip_names(def));
    }

    #[test]
    fn renaming_binders_never_changes_the_id(body in arb_term()) {
        let def = Definition::test(body);
        let a = normalise_one(&def).unwrap().id;
        let b = normalise_one(&rename_all(def, "q")).unwrap().id;
        prop_assert_eq!(a, b);
    }

    #[test]
    fn decoding_garbage_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..256)) {
        let _ = decode(&bytes);
    }
}
