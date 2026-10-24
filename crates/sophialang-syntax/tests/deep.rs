//! SP-015 / SR-145: terms of any depth. Every pass over a term (encode,
//! decode, clone, compare, hash, Debug, walk, map, print, parse, elaborate,
//! drop) runs at depth 100 000 on a 256 KiB thread stack; the stack
//! segments it needs come from `sophia_hash::deep::grow`.

use sophialang_core::samples::builtin;
use sophialang_core::*;
use sophialang_syntax::*;

const DEPTH: usize = 100_000;

fn on_small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new().stack_size(256 * 1024).spawn(f).unwrap().join().unwrap();
}

/// `0 + 1 + 1 + …`: a left-nested application chain, as a long polynomial.
fn sum_chain(n: usize) -> Term {
    let plus = builtin("+");
    let mut t = Term::Lit(Lit::Int64(0));
    for _ in 0..n {
        t = Term::app(Term::Nominal(plus), [t, Term::Lit(Lit::Int64(1))]);
    }
    t
}

/// `_ -> _ -> … -> 0`: right-nested lambdas.
fn lambda_chain(n: usize) -> Term {
    let mut t = Term::Lit(Lit::Int64(0));
    for _ in 0..n {
        t = Term::lam("x", t);
    }
    t
}

/// `let y = 1 … let y = 1 … 0`: nested lets (values and bodies).
fn let_chain(n: usize) -> Term {
    let mut t = Term::Lit(Lit::Int64(0));
    for _ in 0..n {
        t = Term::Let { binder: Binder::named("y"), ty: None, value: Box::new(Term::Lit(Lit::Int64(1))), body: Box::new(t) };
    }
    t
}

#[test]
fn core_passes_at_depth_100_000() {
    on_small_stack(|| {
        for t in [sum_chain(DEPTH), lambda_chain(DEPTH), let_chain(DEPTH)] {
            let def = Definition::test(t);
            let r = normalise_one(&def).unwrap();
            let back = definition_of(&r).unwrap();
            assert!(back == def.clone(), "decode inverts encode; clone and eq work");
            let mut h = std::collections::hash_map::DefaultHasher::new();
            std::hash::Hash::hash(&back, &mut h);
            assert!(format!("{back:?}").len() > DEPTH);
            let mut n = 0;
            back.walk(&mut |_| n += 1);
            assert!(n > DEPTH);
            let mapped = back.map_terms(&mut |t| t);
            assert_eq!(normalise_one(&mapped).unwrap().id, r.id);
        }
    });
}

#[test]
fn print_and_parse_at_depth_100_000() {
    on_small_stack(|| {
        let big = Limits { max_bytes: 64 << 20, max_nodes: 1 << 24, ..Limits::default() };
        for (t, profile) in [(sum_chain(DEPTH), Profile::julia()), (lambda_chain(DEPTH), Profile::built_in("rust").unwrap())] {
            let def = Definition::test(t);
            let id = normalise_one(&def).unwrap().id;
            let text = print(&EmptyEnv, "", &profile, &[ToPrint { name: "t", def: &def, id: Some(id), index: 0, relation: false }]);
            let (items, _) = parse_text(&EmptyEnv, "", &text, big).unwrap_or_else(|e| panic!("{:?}", e[0]));
            assert_eq!(normalise_one(items[0].definition()).unwrap().id, id, "{} round trip", profile.name);
        }
    });
}

#[test]
#[ignore]
fn probe_print_parse_costs() {
    on_small_stack(|| {
        let big = Limits { max_bytes: 64 << 20, max_nodes: 1 << 24, ..Limits::default() };
        for (name, t) in [("sum", sum_chain(DEPTH)), ("lambda", lambda_chain(DEPTH)), ("let", let_chain(DEPTH))] {
            let def = Definition::test(t);
            let id = normalise_one(&def).unwrap().id;
            let t0 = std::time::Instant::now();
            let text = print(&EmptyEnv, "", &Profile::julia(), &[ToPrint { name: "t", def: &def, id: Some(id), index: 0, relation: false }]);
            let t1 = t0.elapsed();
            let t0 = std::time::Instant::now();
            let ok = parse_text(&EmptyEnv, "", &text, big).is_ok();
            eprintln!("{name}: {} bytes, print {t1:?}, parse {:?} ok={ok}", text.len(), t0.elapsed());
        }
    });
}
