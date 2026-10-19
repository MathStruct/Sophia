//! A small sample program, used by tests, benchmarks and the sample store
//! (SPEC-004 §9). It covers every definition kind, recursion, mutual
//! recursion, a hole, an unresolved name, an `Unsafe` section and nominal
//! references. See `samples.md`.

use sophia_hash::{Digest, NominalId};

use crate::definition::{Body, Constructor, Definition, Field, Kind};
use crate::term::{Aspect, Binder, Case, Lit, Term};

/// The nominal id of a built-in entity (`Int64`, `+`, `==`, …). Built-ins
/// have intrinsic identities, derived from their names, as in Unison.
pub fn builtin(name: &str) -> NominalId {
    let nonce = *Digest::of(format!("sophialang-builtin:{name}").as_bytes()).as_bytes();
    sophia_hash::merkle::nominal_id(crate::LANG, "builtin", &nonce)
}

fn d(i: u32) -> Term {
    Term::Draft(i)
}

fn con(ty: Term, index: u32) -> Term {
    Term::Con { ty: Box::new(ty), index }
}

fn matching(scrutinee: Term, cases: Vec<Case>) -> Term {
    Term::Match { scrutinee: Box::new(scrutinee), motive: None, cases }
}

fn case(constructor: u32, binders: &[&str], body: Term) -> Case {
    Case { constructor, binders: binders.iter().map(|b| Binder::named(*b)).collect(), body }
}

/// The sample program as (name, definition) pairs. `Term::Draft(i)` refers
/// to the i-th entry.
pub fn program() -> Vec<(&'static str, Definition)> {
    let nat = || d(0);
    let int64 = || Term::Nominal(builtin("Int64"));
    let bool_t = || Term::Nominal(builtin("Bool"));
    vec![
        // 0
        (
            "sample.Nat",
            Definition::data(
                Term::Type(0),
                vec![
                    Constructor { name: "zero".into(), ty: nat() },
                    Constructor { name: "succ".into(), ty: Term::pi("n", nat(), nat()) },
                ],
            ),
        ),
        // 1: add(a, b) = match a { zero => b; succ(k) => succ(add(k, b)) }
        (
            "sample.add",
            Definition::method(
                Term::pi("a", nat(), Term::pi("b", nat(), nat())),
                Term::lam(
                    "a",
                    Term::lam(
                        "b",
                        matching(
                            Term::Var(1),
                            vec![
                                case(0, &[], Term::Var(0)),
                                case(1, &["k"], Term::app(con(nat(), 1), [Term::app(d(1), [Term::Var(0), Term::Var(1)])])),
                            ],
                        ),
                    ),
                ),
            ),
        ),
        // 2, 3: even and odd, mutually recursive
        (
            "sample.even",
            Definition::method(
                Term::arrow(nat(), bool_t()),
                Term::lam(
                    "n",
                    matching(
                        Term::Var(0),
                        vec![case(0, &[], Term::Lit(Lit::Bool(true))), case(1, &["k"], Term::app(d(3), [Term::Var(0)]))],
                    ),
                ),
            ),
        ),
        (
            "sample.odd",
            Definition::method(
                Term::arrow(nat(), bool_t()),
                Term::lam(
                    "n",
                    matching(
                        Term::Var(0),
                        vec![case(0, &[], Term::Lit(Lit::Bool(false))), case(1, &["k"], Term::app(d(2), [Term::Var(0)]))],
                    ),
                ),
            ),
        ),
        // 4: a definition with a hole
        ("sample.todo", Definition::method(Term::arrow(nat(), nat()), Term::lam("n", Term::hole(nat())))),
        // 5: a definition with an unresolved name
        (
            "sample.norm",
            Definition::method(
                Term::arrow(int64(), int64()),
                Term::lam("x", Term::app(Term::Unresolved(vec!["math".into(), "sqrt".into()]), [Term::Var(0)])),
            ),
        ),
        // 6, 7: relations
        (
            "sample.parent",
            Definition {
                kind: Kind::Relation,
                ty: Some(Term::arrow(nat(), Term::arrow(nat(), Term::Prop))),
                body: Body::None,
            },
        ),
        (
            "sample.ancestor",
            Definition {
                kind: Kind::Relation,
                ty: Some(Term::arrow(nat(), Term::arrow(nat(), Term::Prop))),
                body: Body::None,
            },
        ),
        // 8: ancestor(X, Y) :- parent(X, Y)
        (
            "sample.ancestor_base",
            Definition {
                kind: Kind::Rule,
                ty: None,
                body: Body::Rule {
                    vars: vec![Binder::named("X"), Binder::named("Y")],
                    head: Term::app(d(7), [Term::Var(1), Term::Var(0)]),
                    premises: vec![Term::app(d(6), [Term::Var(1), Term::Var(0)])],
                    weight: None,
                },
            },
        ),
        // 9: ancestor(X, Z) :- parent(X, Y), ancestor(Y, Z)   [weighted]
        (
            "sample.ancestor_step",
            Definition {
                kind: Kind::Rule,
                ty: None,
                body: Body::Rule {
                    vars: vec![Binder::named("X"), Binder::named("Y"), Binder::named("Z")],
                    head: Term::app(d(7), [Term::Var(2), Term::Var(0)]),
                    premises: vec![
                        Term::app(d(6), [Term::Var(2), Term::Var(1)]),
                        Term::app(d(7), [Term::Var(1), Term::Var(0)]),
                    ],
                    weight: Some(Lit::float(0.9)),
                },
            },
        ),
        // 10: a test
        (
            "sample.add_test",
            Definition::test(Term::app(
                Term::Nominal(builtin("==")),
                [
                    Term::app(d(1), [con(nat(), 0), con(nat(), 0)]),
                    con(nat(), 0),
                ],
            )),
        ),
        // 11: a method on a machine type: double(x::Int64)::Int64 = x + x
        (
            "sample.double",
            Definition::method(
                Term::arrow(int64(), int64()),
                Term::lam("x", Term::app(Term::Nominal(builtin("+")), [Term::Var(0), Term::Var(0)])),
            ),
        ),
        // 12: an explicit (unsafe) memory section
        (
            "sample.peek",
            Definition::method(
                Term::arrow(Term::Nominal(builtin("Ptr")), int64()),
                Term::lam("p", Term::Unsafe(Aspect::Memory, Box::new(Term::app(Term::prim("load_int64"), [Term::Var(0)])))),
            ),
        ),
        // 13: a postulate: declared, not written
        ("sample.sort_spec", Definition::postulate(Term::arrow(nat(), Term::Prop))),
        // 14: a struct layout
        (
            "sample.CircleLayout",
            Definition {
                kind: Kind::Layout,
                ty: None,
                body: Body::Fields(vec![Field { name: "r".into(), ty: Term::Nominal(builtin("Float64")) }]),
            },
        ),
        // 15: a constant
        ("sample.two", Definition::constant(nat(), Term::app(con(nat(), 1), [Term::app(con(nat(), 1), [con(nat(), 0)])]))),
    ]
}
