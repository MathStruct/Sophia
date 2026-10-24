//! SP-015 on wasm: every pass over terms at depth 100 000 in a wasm engine
//! (Node or a browser), where the stack cannot grow: encoding, decoding,
//! display names, equality, cloning, hashing, `Debug`, printing, parsing,
//! elaborating and dropping all use explicit stacks.
#![cfg(target_arch = "wasm32")]

use sophialang_core::samples::builtin;
use sophialang_core::*;
use sophialang_syntax::*;
use wasm_bindgen_test::wasm_bindgen_test;

const DEPTH: usize = 100_000;

fn sum_chain(n: usize) -> Term {
    let plus = builtin("+");
    let mut t = Term::Lit(Lit::Int64(0));
    for _ in 0..n {
        t = Term::app(Term::Nominal(plus), [t, Term::Lit(Lit::Int64(1))]);
    }
    t
}

fn lambda_chain(n: usize) -> Term {
    let mut t = Term::Var(0);
    for _ in 0..n {
        t = Term::lam("x", t);
    }
    t
}

fn let_chain(n: usize) -> Term {
    let mut t = Term::Lit(Lit::Int64(0));
    for _ in 0..n {
        t = Term::Let { binder: Binder::named("y"), ty: None, value: Box::new(Term::Lit(Lit::Int64(1))), body: Box::new(t) };
    }
    t
}

fn read_path(t: Term) {
    let def = Definition::test(t);
    let r = normalise_one(&def).unwrap();
    let back = definition_of(&r).unwrap();
    assert!(back == def.clone(), "decode inverts encode");
    let mut h = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&back, &mut h);
    assert!(format!("{back:?}").len() > DEPTH);
    let big = Limits { max_bytes: 256 << 20, max_nodes: 1 << 24, ..Limits::default() };
    for profile in [Profile::julia(), Profile::built_in("rust").unwrap()] {
        let text = print(&EmptyEnv, "", &profile, &[ToPrint { name: "t", def: &back, id: Some(r.id), index: 0, relation: false }]);
        let (items, _) = parse_text(&EmptyEnv, "", &text, big).unwrap_or_else(|e| panic!("{:?}", e[0]));
        assert_eq!(normalise_one(items[0].definition()).unwrap().id, r.id, "{} round trip", profile.name);
    }
}

#[wasm_bindgen_test]
fn sum_chain_100_000() {
    read_path(sum_chain(DEPTH));
}

#[wasm_bindgen_test]
fn lambda_chain_100_000() {
    read_path(lambda_chain(DEPTH));
}

#[wasm_bindgen_test]
fn let_chain_100_000() {
    read_path(let_chain(DEPTH));
}
