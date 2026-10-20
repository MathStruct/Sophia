//! Property tests (review of M3a, §7 and §8):
//! - canonical form is invariant under predicate permutation, rule order,
//!   duplicate rules, unreachable predicates and renaming, and survives an
//!   encode/decode round trip;
//! - the magic-set rewrite preserves meaning on random safe programs over a
//!   random store: plain, magic and SQL evaluation agree.

use std::collections::BTreeSet;

use sophia_hash::{Digest, Record, Status};
use sophia_query::ir::*;
use sophia_query::{Error, Limits, Strategy, canon, prepare};
use sophia_store::{Backend, Store, Visibility};

struct Rng(u64);
impl Rng {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) % n.max(1)
    }
    fn chance(&mut self, pct: u64) -> bool {
        self.below(100) < pct
    }
}

fn id(i: u64) -> Digest {
    Digest::of(&i.to_le_bytes())
}

const NODES: u64 = 40;

/// A random query over `refs` and `def`, with 1–4 local predicates of arity
/// 1 or 2, recursion (including mutual), negation, (in)equalities, repeated
/// variables, constants, an Id parameter, and sometimes a count.
fn random_query(rng: &mut Rng) -> (Query, Names) {
    let np = 1 + rng.below(4) as usize;
    let preds: Vec<u32> = (0..np).map(|_| 1 + rng.below(2) as u32).collect();
    let var = |rng: &mut Rng| Term::Var(rng.below(3) as u32);
    let mut rules = Vec::new();
    for p in 0..np {
        for _ in 0..1 + rng.below(3) {
            let mut body = Vec::new();
            for _ in 0..1 + rng.below(3) {
                body.push(match rng.below(10) {
                    0..=3 => Literal::Pos(Atom { pred: Pred::Base(Base::Refs), args: vec![var(rng), var(rng)] }),
                    4 => Literal::Pos(Atom {
                        pred: Pred::Base(Base::Def),
                        args: vec![var(rng), Term::Const(Val::Text(["method", "data"][rng.below(2) as usize].into()))],
                    }),
                    5 => Literal::Pos(Atom {
                        pred: Pred::Base(Base::Refs),
                        args: vec![var(rng), Term::Const(Val::Id(id(rng.below(NODES))))],
                    }),
                    _ => {
                        let q = rng.below(np as u64) as u32;
                        Literal::Pos(Atom { pred: Pred::Local(q), args: (0..preds[q as usize]).map(|_| var(rng)).collect() })
                    }
                });
            }
            // Variables bound by the positive literals so far: head
            // arguments, negations and inequalities draw from them, so most
            // programs are safe.
            let bound: Vec<u32> = {
                let mut b: Vec<u32> = body.iter().flat_map(|l| l.vars()).collect();
                b.sort();
                b.dedup();
                b
            };
            let bvar = |rng: &mut Rng| Term::Var(bound[rng.below(bound.len() as u64) as usize]);
            if rng.chance(25) {
                let q = rng.below(np as u64) as u32;
                let args = (0..preds[q as usize]).map(|_| bvar(rng)).collect();
                body.push(Literal::Neg(Atom { pred: Pred::Local(q), args }));
            }
            if rng.chance(15) {
                body.push(Literal::Ne(bvar(rng), bvar(rng)));
            }
            if rng.chance(10) {
                body.push(Literal::Neg(Atom { pred: Pred::Base(Base::Refs), args: vec![bvar(rng), Term::Var(3)] }));
            }
            let args = (0..preds[p]).map(|_| bvar(rng)).collect();
            rules.push(Rule { head: p as u32, args, body, vars: 4 });
        }
    }
    let target = if rng.chance(50) { Term::Param(0) } else { Term::Const(Val::Id(id(rng.below(NODES)))) };
    let a0 = preds[0];
    let args = if a0 == 2 {
        if rng.chance(50) { vec![Term::Var(0), target] } else { vec![target, Term::Var(0)] }
    } else {
        vec![Term::Var(0)]
    };
    let mut sbody = vec![Literal::Pos(Atom { pred: Pred::Local(0), args })];
    if rng.chance(20) {
        sbody.push(Literal::Pos(Atom { pred: Pred::Base(Base::Def), args: vec![Term::Var(0), Term::Var(1)] }));
    }
    let outputs = if sbody.len() == 2 && rng.chance(50) {
        vec![Output::Var(1), Output::Count(0)]
    } else {
        vec![Output::Var(0)]
    };
    let q = Query { params: vec![Sort::Id], preds, rules, select: Select { outputs, body: sbody, vars: 2 } };
    let names = Names {
        query: "q".into(),
        params: vec!["t".into()],
        preds: (0..np).map(|i| format!("p{i}")).collect(),
        rule_vars: q.rules.iter().map(|_| vec!["a".into(), "b".into(), "c".into(), "d".into()]).collect(),
        select_vars: vec!["x".into(), "k".into()],
    };
    (q, names)
}

fn canonical_bytes(q: &Query, n: &Names) -> Result<Vec<u8>, Error> {
    canon::canonicalise(q, n).map(|(c, _)| canon::encode(&c))
}

/// Applies a random permutation of predicates, shuffles rules, duplicates
/// one, adds an unreachable predicate, and renames everything.
fn scramble(q: &Query, n: &Names, rng: &mut Rng) -> (Query, Names) {
    let np = q.preds.len();
    let mut perm: Vec<u32> = (0..np as u32).collect();
    for i in (1..np).rev() {
        perm.swap(i, rng.below(i as u64 + 1) as usize);
    }
    let map_body = |b: &[Literal]| -> Vec<Literal> {
        b.iter()
            .map(|l| match l {
                Literal::Pos(Atom { pred: Pred::Local(p), args }) => Literal::Pos(Atom { pred: Pred::Local(perm[*p as usize]), args: args.clone() }),
                Literal::Neg(Atom { pred: Pred::Local(p), args }) => Literal::Neg(Atom { pred: Pred::Local(perm[*p as usize]), args: args.clone() }),
                l => l.clone(),
            })
            .collect()
    };
    let mut preds = vec![0; np];
    for p in 0..np {
        preds[perm[p] as usize] = q.preds[p];
    }
    let mut rules: Vec<Rule> = q
        .rules
        .iter()
        .map(|r| Rule { head: perm[r.head as usize], args: r.args.clone(), body: map_body(&r.body), vars: r.vars })
        .collect();
    let dup = rules[rng.below(rules.len() as u64) as usize].clone();
    rules.push(dup);
    for i in (1..rules.len()).rev() {
        rules.swap(i, rng.below(i as u64 + 1) as usize);
    }
    // An unreachable predicate.
    preds.push(1);
    rules.push(Rule {
        head: np as u32,
        args: vec![Term::Var(0)],
        body: vec![Literal::Pos(Atom { pred: Pred::Base(Base::Refs), args: vec![Term::Var(0), Term::Var(1)] })],
        vars: 2,
    });
    let select = Select { body: map_body(&q.select.body), ..q.select.clone() };
    let names = Names {
        query: "renamed".into(),
        params: vec!["other".into()],
        preds: (0..=np).map(|i| format!("r{i}")).collect(),
        rule_vars: rules.iter().map(|_| vec!["u".into(), "v".into(), "w".into(), "z".into()]).collect(),
        select_vars: vec!["y".into(), "j".into()],
    };
    let _ = n;
    (Query { params: q.params.clone(), preds, rules, select }, names)
}

#[test]
fn canonical_form_is_invariant_and_round_trips() {
    let cases: u64 = std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(2000);
    let mut refused = 0;
    for seed in 0..cases {
        let mut rng = Rng(seed * 7919 + 1);
        let (q, n) = random_query(&mut rng);
        let base = match canonical_bytes(&q, &n) {
            Ok(b) => b,
            Err(_) => {
                refused += 1;
                continue;
            }
        };
        for _ in 0..4 {
            let (q2, n2) = scramble(&q, &n, &mut rng);
            assert_eq!(canonical_bytes(&q2, &n2).unwrap(), base, "seed {seed}: canonical form changed under scrambling");
        }
        let (c, _) = canon::canonicalise(&q, &n).unwrap();
        assert_eq!(canon::decode(&base).unwrap(), c, "seed {seed}: decode(encode) differs");
        assert_eq!(canon::canonicalise(&c, &n).map(|(x, _)| canon::encode(&x)).unwrap(), base, "seed {seed}: not idempotent");
    }
    assert!(refused * 100 < cases, "too many refused as too symmetric: {refused}");
}

#[test]
fn symmetric_predicates_get_one_id() {
    // q and r are interchangeable: every ordering of p's rules is tried.
    let a = sophia_query::prepare_text(
        "query s\n p(x) = q(x)\n p(x) = r(x)\n q(x) = def(x, :a) && r(x)\n r(x) = def(x, :a) && q(x)\n select x where p(x)\nend",
    )
    .unwrap();
    let b = sophia_query::prepare_text(
        "query s\n p(x) = r(x)\n p(x) = q(x)\n r(x) = def(x, :a) && q(x)\n q(x) = def(x, :a) && r(x)\n select x where p(x)\nend",
    )
    .unwrap();
    assert_eq!(a.id, b.id);
}

fn populate<B: Backend>(s: &Store<B>, seed: u64) {
    let mut rng = Rng(seed);
    let records: Vec<Record> = (0..NODES)
        .map(|i| {
            let mut refs: Vec<Digest> = (0..rng.below(4)).map(|_| id(rng.below(NODES))).collect();
            refs.sort();
            refs.dedup();
            Record {
                id: id(i),
                lang: "test".into(),
                normaliser: "test@0".into(),
                kind: ["method", "data"][rng.below(2) as usize].into(),
                component: id(i),
                index: 0,
                component_size: 1,
                bytes: i.to_le_bytes().to_vec(),
                display: Vec::new(),
                status: Status::Complete,
                refs,
                nominal_refs: Vec::new(),
            }
        })
        .collect();
    let names: Vec<(String, usize, Visibility)> = (0..NODES as usize).step_by(4).map(|i| (format!("n.n{i}"), i, Visibility::Public)).collect();
    s.add(&records, &names).unwrap();
}

#[test]
fn magic_sets_preserve_meaning_on_random_programs() {
    let cases: u64 = std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(1500);
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("p.sqlite")).unwrap());
    populate(&s, 99);
    let (mut checked, mut rewritten, mut via_sql) = (0, 0, 0);
    let limits = Limits { max_rows: usize::MAX, ..Default::default() };
    for seed in 0..cases {
        let mut rng = Rng(seed * 104729 + 3);
        let (q, n) = random_query(&mut rng);
        let Ok(p) = prepare(&q, &n) else { continue };
        checked += 1;
        rewritten += p.magic.is_some() as u64;
        for t in [0, 7, 21] {
            let params = [Val::Id(id(t))];
            let plain: BTreeSet<Vec<Val>> = s
                .read(|tx| Ok(p.run(tx, &params, limits, Strategy::Plain).unwrap()))
                .unwrap()
                .rows
                .into_iter()
                .collect();
            let magic: BTreeSet<Vec<Val>> = s
                .read(|tx| Ok(p.run(tx, &params, limits, Strategy::Auto).unwrap()))
                .unwrap()
                .rows
                .into_iter()
                .collect();
            assert_eq!(plain, magic, "seed {seed}, t {t}\n{}\n{}", p.print(sophia_query::Profile::Julia), p.explain(Strategy::Auto));
            for st in [Strategy::Plain, Strategy::Auto] {
                match sophia_query::run_sql(s.backend(), &p, &params, limits, st) {
                    Ok(r) => {
                        via_sql += 1;
                        let sql: BTreeSet<Vec<Val>> = r.rows.into_iter().collect();
                        assert_eq!(sql, plain, "seed {seed}, t {t}, {st:?}\n{}\n{}", p.print(sophia_query::Profile::Julia), p.to_sql(&params, st).unwrap());
                    }
                    Err(Error::Unsupported(_)) => {}
                    Err(e) => panic!("seed {seed}: {e}\n{}", p.to_sql(&params, st).unwrap_or_default()),
                }
            }
        }
    }
    eprintln!("{checked} valid programs, {rewritten} rewritten by magic sets, {via_sql} SQL runs");
    assert!(checked > cases / 5, "the generator should produce enough valid programs: {checked}");
    assert!(rewritten > checked / 4, "enough programs exercise the rewrite: {rewritten}");
}
