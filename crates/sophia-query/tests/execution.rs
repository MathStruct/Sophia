//! Execution (SPEC-007 §6, §7): every query gives the same rows with the
//! plain and the magic-set program, on every backend, and through SQL on
//! SQLite; results are checked against direct graph walks.

use std::collections::{BTreeSet, HashSet};

use sophia_hash::{Digest, Record, Status};
use sophia_query::{Cache, Error, Limits, Prepared, Strategy, Val, prepare_text};
use sophia_store::{Backend, ReadTx, Store, Visibility};

struct Rng(u64);
impl Rng {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) % n
    }
}

fn id(i: u64) -> Digest {
    Digest::of(&i.to_le_bytes())
}

fn fake(i: u64, kind: &str, refs: Vec<Digest>) -> Record {
    Record {
        id: id(i),
        lang: "test".into(),
        normaliser: "test@0".into(),
        kind: kind.into(),
        component: id(i),
        index: 0,
        component_size: 1,
        bytes: i.to_le_bytes().to_vec(),
        display: Vec::new(),
        status: Status::Complete,
        refs,
        nominal_refs: Vec::new(),
    }
}

/// A random reference graph with cycles, names on every third node, and
/// one generic function with a few methods.
fn populate<B: Backend>(s: &Store<B>, seed: u64, n: u64) {
    let mut rng = Rng(seed);
    let kinds = ["method", "constant", "data"];
    let records: Vec<Record> = (0..n)
        .map(|i| {
            let k = rng.below(4);
            let mut refs: Vec<Digest> = (0..k).map(|_| id(rng.below(n))).filter(|d| *d != id(i)).collect();
            refs.sort();
            refs.dedup();
            fake(i, kinds[rng.below(3) as usize], refs)
        })
        .collect();
    let names: Vec<(String, usize, Visibility)> =
        (0..n as usize).step_by(3).map(|i| (format!("g.n{i}"), i, Visibility::Public)).collect();
    s.add(&records, &names).unwrap();
    let f = s.new_nominal("test", "generic", "g.f").unwrap();
    for i in (0..n).step_by(5) {
        s.add_method(&f, &id(i)).unwrap();
    }
}

fn transitive_dependants(tx: &dyn ReadTx, target: &Digest) -> BTreeSet<Digest> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![*target];
    while let Some(d) = stack.pop() {
        for x in tx.dependants(&d).unwrap() {
            if seen.insert(x) {
                stack.push(x);
            }
        }
    }
    seen
}

const QUERIES: [&str; 8] = [
    // right-linear transitive dependants
    "query reach(t::Id)\n dep(a, b) = refs(a, b)\n dep(a, c) = refs(a, b) && dep(b, c)\n select a where dep(a, t)\nend",
    // left-linear transitive dependencies
    "query down(t::Id)\n dep(a, b) = refs(a, b)\n dep(a, c) = dep(a, b) && refs(b, c)\n select c where dep(t, c)\nend",
    // same generation
    "query sg(t::Id)\n sg(a, b) = refs(a, p) && refs(b, p) && a != b\n sg(a, b) = refs(a, p) && refs(b, q) && sg(p, q)\n select b where sg(t, b)\nend",
    // negation: reachable definitions that reference nothing
    "query leaves(t::Id)\n down(a) = refs(t, a)\n down(b) = down(a) && refs(a, b)\n select a where down(a) && !refs(a, _)\nend",
    // grouped count
    "query kinds(t::Id)\n select k, count(d) where def(d, k) && d != t\nend",
    // names by prefix
    "query named(t::Id)\n select p, d where name(p, d) && startswith(p, \"g.n1\") && d != t\nend",
    // a generic function's methods, through its name
    "query methods(t::Id)\n select d, k where name(\"g.f\", f) && member(f, d) && def(d, k) && d != t\nend",
    // mutual recursion: paths of even length (no SQL translation)
    "query even(t::Id)\n odd(a, b) = refs(a, b)\n odd(a, c) = even(a, b) && refs(b, c)\n even(a, c) = odd(a, b) && refs(b, c)\n select b where even(t, b)\nend",
];

fn run(p: &Prepared, tx: &dyn ReadTx, t: Digest, s: Strategy) -> BTreeSet<Vec<Val>> {
    p.run(tx, &[Val::Id(t)], Limits::default(), s).unwrap().rows.into_iter().collect()
}

fn plain_and_magic_agree<B: Backend>(s: Store<B>) {
    let n = 120;
    populate(&s, 7, n);
    let prepared: Vec<Prepared> = QUERIES.iter().map(|q| prepare_text(q).unwrap()).collect();
    for (qi, p) in prepared.iter().enumerate() {
        if qi < 3 || qi == 7 {
            assert!(p.magic.is_some(), "query {qi} gets a magic-set rewrite");
        }
        for t in [0, 1, 17, 50, 99] {
            s.read(|tx| {
                let plain = run(p, tx, id(t), Strategy::Plain);
                let magic = run(p, tx, id(t), Strategy::Auto);
                assert_eq!(plain, magic, "query {qi}, target {t}\n{}", p.explain(Strategy::Auto));
                if qi == 0 {
                    let expect: BTreeSet<Vec<Val>> =
                        transitive_dependants(tx, &id(t)).into_iter().map(|d| vec![Val::Id(d)]).collect();
                    assert_eq!(magic, expect, "reach matches a graph walk");
                }
                Ok(())
            })
            .unwrap();
        }
    }
}

fn magic_sets_read_less<B: Backend>(s: Store<B>) {
    populate(&s, 11, 400);
    let p = prepare_text(QUERIES[0]).unwrap();
    s.read(|tx| {
        let plain = p.run(tx, &[Val::Id(id(3))], Limits::default(), Strategy::Plain).unwrap();
        let magic = p.run(tx, &[Val::Id(id(3))], Limits::default(), Strategy::Auto).unwrap();
        assert_eq!(plain.clone().sorted().rows, magic.clone().sorted().rows);
        assert!(magic.stats.derived < plain.stats.derived, "{:?} vs {:?}", magic.stats, plain.stats);
        assert_eq!(magic.stats.scans, 0, "goal-directed: no full scans");
        Ok(())
    })
    .unwrap();
}

fn results_are_cached_per_snapshot<B: Backend>(s: Store<B>) {
    populate(&s, 3, 60);
    let p = prepare_text(QUERIES[0]).unwrap();
    let params = [Val::Id(id(5))];
    let (a, rid1, c1) = sophia_query::run_cached(&s, &p, &params, Limits::default()).unwrap();
    let (b, rid2, c2) = sophia_query::run_cached(&s, &p, &params, Limits::default()).unwrap();
    assert_eq!((c1, c2), (Cache::Miss, Cache::Hit));
    assert_eq!(rid1, rid2);
    assert_eq!(a.sorted().rows, b.sorted().rows);
    // Another parameter is another result.
    let (_, rid3, c3) = sophia_query::run_cached(&s, &p, &[Val::Id(id(6))], Limits::default()).unwrap();
    assert_ne!(rid3, rid1);
    assert_eq!(c3, Cache::Miss);
    // A change to the store is a new snapshot.
    s.add(&[fake(1000, "method", vec![id(5)])], &[]).unwrap();
    let (c, rid4, c4) = sophia_query::run_cached(&s, &p, &params, Limits::default()).unwrap();
    assert_ne!(rid4, rid1);
    assert_eq!(c4, Cache::Miss);
    assert!(c.rows.contains(&vec![Val::Id(id(1000))]));
}

fn queries_are_stored_as_definitions<B: Backend>(s: Store<B>) {
    populate(&s, 5, 30);
    let text = format!("select a where refs(a, 0x{})", id(4).to_hex());
    let p = prepare_text(&text).unwrap();
    let qid = sophia_query::save(&s, &p, Some("queries.uses_n4")).unwrap();
    assert_eq!(qid, p.id);
    let back = sophia_query::load(&s, "queries.uses_n4").unwrap();
    assert_eq!(back.id, p.id);
    assert_eq!(back.print(sophia_query::Profile::Julia), p.print(sophia_query::Profile::Julia));
    // The query references the definition it names, so it is a dependant.
    assert!(s.read(|tx| tx.dependants(&id(4))).unwrap().contains(&qid));
    // And it is itself queryable.
    let qs = prepare_text("select q where def(q, :Query)").unwrap();
    let rows = s.read(|tx| Ok(qs.run(tx, &[], Limits::default(), Strategy::Auto).unwrap())).unwrap();
    assert_eq!(rows.rows, vec![vec![Val::Id(qid)]]);
    assert!(s.verify(&qid).unwrap());
}

fn limits_stop_runaway_queries<B: Backend>(s: Store<B>) {
    populate(&s, 9, 200);
    let p = prepare_text("select a, b where refs(a, x) && refs(b, y)").unwrap();
    let r = s.read(|tx| Ok(p.run(tx, &[], Limits { max_rows: usize::MAX, max_steps_k: 1, ..Default::default() }, Strategy::Auto))).unwrap();
    assert!(matches!(r, Err(Error::Limit(_))), "{r:?}");
    // Memory is bounded too: the closure of the whole store, without magic
    // sets, stops at the derived-tuple cap.
    let reach = prepare_text(QUERIES[0]).unwrap();
    let r = s
        .read(|tx| Ok(reach.run(tx, &[Val::Id(id(1))], Limits { max_derived: 50, ..Default::default() }, Strategy::Plain)))
        .unwrap();
    assert!(matches!(&r, Err(Error::Limit(m)) if m.contains("derived")), "{r:?}");
    // The row cap stops the select early and marks the result truncated.
    let all = prepare_text("select a, b where refs(a, b)").unwrap();
    let r = s.read(|tx| Ok(all.run(tx, &[], Limits { max_rows: 5, ..Default::default() }, Strategy::Auto).unwrap())).unwrap();
    assert_eq!(r.rows.len(), 5);
    assert!(r.truncated);
}

macro_rules! suite {
    ($module:ident, $make:expr) => {
        mod $module {
            use super::*;
            fn store() -> (Store<impl Backend>, tempfile::TempDir) {
                let dir = tempfile::tempdir().unwrap();
                let make = $make;
                (Store::new(make(dir.path())), dir)
            }
            suite!(@tests plain_and_magic_agree, magic_sets_read_less, results_are_cached_per_snapshot,
                queries_are_stored_as_definitions, limits_stop_runaway_queries);
        }
    };
    (@tests $($t:ident),*) => {
        $(#[test] fn $t() { let (s, _d) = store(); super::$t(s); })*
    };
}

suite!(sqlite, |p: &std::path::Path| sophia_store::sqlite::Sqlite::open(p.join("store.sqlite")).unwrap());
suite!(redb, |p: &std::path::Path| sophia_store::redb_store::Redb::open(p.join("store.redb")).unwrap());
#[cfg(feature = "lmdb")]
suite!(lmdb, |p: &std::path::Path| sophia_store::lmdb_store::Lmdb::open_with_map_size(p.join("store.lmdb"), 1 << 30).unwrap());
#[cfg(feature = "duckdb")]
suite!(duckdb, |p: &std::path::Path| sophia_store::duckdb_store::DuckDb::open(p.join("store.duckdb")).unwrap());

/// The SQL translation gives the native executor's rows, for both programs.
#[test]
fn sql_translation_agrees_with_the_native_executor() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("q.sqlite")).unwrap());
    populate(&s, 7, 120);
    for (qi, text) in QUERIES.iter().enumerate() {
        let p = prepare_text(text).unwrap();
        for t in [0, 1, 17, 50, 99] {
            let params = [Val::Id(id(t))];
            let native: HashSet<Vec<Val>> =
                s.read(|tx| Ok(p.run(tx, &params, Limits::default(), Strategy::Auto).unwrap())).unwrap().rows.into_iter().collect();
            for strategy in [Strategy::Plain, Strategy::Auto] {
                match sophia_query::run_sql(s.backend(), &p, &params, Limits::default(), strategy) {
                    Ok(rows) => {
                        let sql: HashSet<Vec<Val>> = rows.rows.into_iter().collect();
                        assert_eq!(sql, native, "query {qi}, target {t}, {strategy:?}\n{}", p.to_sql(&params, strategy).unwrap());
                    }
                    Err(Error::Unsupported(_)) => assert_eq!(qi, 7, "only mutual recursion lacks a translation"),
                    Err(e) => panic!("query {qi}: {e}\n{:?}", p.to_sql(&params, strategy)),
                }
            }
        }
    }
}

#[test]
fn compiled_sql_cannot_write() {
    let dir = tempfile::tempdir().unwrap();
    let db = sophia_store::sqlite::Sqlite::open(dir.path().join("q.sqlite")).unwrap();
    let l = sophia_store::query::QueryLimits::default();
    assert!(db.query_compiled("DELETE FROM t_ref", l).is_err());
    assert!(db.query_compiled("SELECT * FROM sqlite_master", l).is_err());
    assert!(db.query_compiled("SELECT count(*) FROM t_ref", l).is_ok());
}
