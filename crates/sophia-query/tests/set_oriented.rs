//! The set-oriented access paths (B-014): the prefix access path on names
//! and anti-joins for negated base literals give the same rows as the
//! tuple-at-a-time executor, on every backend, and the same rows as the SQL
//! translation on SQLite. Each query runs under three tunings: tuple at a
//! time, the default, and set at a time from the first probe.

use std::collections::BTreeSet;

use sophia_hash::{Digest, Record, Status};
use sophia_query::{Limits, Prepared, Strategy, Tuning, Val, prepare_text};
use sophia_store::{Backend, Store, Visibility};

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

const N: u64 = 200;

/// Definitions with self-references and unreferenced ones, and names whose
/// paths test the prefix range: a prefix that is a whole name and a proper
/// prefix of others (`p.q1`, `p.q10`, `p.q1.c`), a prefix ending in `.`,
/// backquoted operator segments with a `.` inside (``p.`.+` ``), one without
/// a `.` (`p` matches `p`, `p.…` and `pq…`), non-ASCII paths, and
/// a run of 300 names under one prefix, longer than the executor's first
/// pages (64, 128).
fn populate<B: Backend>(s: &Store<B>) {
    let mut rng = Rng(5);
    let kinds = ["method", "constant", "data"];
    let records: Vec<Record> = (0..N)
        .map(|i| {
            let mut refs: Vec<Digest> = (0..rng.below(3)).map(|_| id(rng.below(N))).collect();
            if i % 37 == 0 {
                refs.push(id(i));
            }
            refs.sort();
            refs.dedup();
            Record {
                id: id(i),
                lang: "test".into(),
                normaliser: "test@0".into(),
                kind: kinds[rng.below(3) as usize].into(),
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
    let mut paths: Vec<String> = (0..300).map(|i| format!("p.q{i}")).collect();
    paths.extend(
        ["p", "pq", "pq.r", "p.q1.c", "p.q1.c.d", "p.`.+`", "p.`.+`.z", "p.`+`", "r.ä", "r.äb", "r.b", "rä", "s.x", "s.x.y"]
            .map(String::from),
    );
    let names: Vec<(String, usize, Visibility)> =
        paths.into_iter().enumerate().map(|(k, p)| (p, (k * 7 % N as usize), Visibility::Public)).collect();
    s.add(&records, &names).unwrap();
}

const PREFIXES: [&str; 16] =
    ["", "p", "p.", "p.`.", "p.`.+`.", "p.q", "p.q1", "p.q10", "p.q299", "p.q3000", "pq", "r.ä", "r", "ä", "s.x", "zzz"];

/// Queries over the new paths. `$` is replaced by a quoted prefix.
const QUERIES: [&str; 13] = [
    "select p where name(p, _) && startswith(p, $)",
    "select p, d where name(p, d) && startswith(p, $) && def(d, :method)",
    "select k, count(p) where name(p, d) && startswith(p, $) && def(d, k)",
    "query q(s::Text)\n select p where name(p, _) && startswith(p, s)\nend",
    "select count(d) where def(d, _) && !refs(_, d)",
    "select k, count(d) where def(d, k) && !refs(d, _)",
    "select a where refs(a, b) && !refs(b, a)",
    "select a where def(a, _) && !refs(a, a)",
    "select a where def(a, _) && !refs(x, x)",
    "select a, k where def(a, k) && !name(_, a) && !def(a, :data)",
    "query r(s::Text)\n named(d) = name(p, d) && startswith(p, s)\n select d where def(d, _) && !named(d) && !refs(d, _)\nend",
    // Negations whose only bound column is a constant.
    "select a where def(a, _) && !def(_, :nothing)",
    "select a where def(a, _) && !def(_, :method)",
];

const TUNINGS: [Tuning; 3] = [
    Tuning::TUPLE_AT_A_TIME,
    // Switches in the middle of a run: the first probes go one at a time.
    Tuning { anti_join_after: 100, prefix_paths: true },
    Tuning { anti_join_after: 0, prefix_paths: true },
];

fn rows(p: &Prepared, mut run: impl FnMut(Tuning) -> sophia_query::Rows) -> BTreeSet<Vec<Val>> {
    let out: Vec<BTreeSet<Vec<Val>>> = TUNINGS.iter().map(|&t| run(t).rows.into_iter().collect()).collect();
    for (t, o) in TUNINGS.iter().zip(&out).skip(1) {
        assert_eq!(&out[0], o, "{t:?}\n{}", p.print(sophia_query::Profile::Julia));
    }
    out.into_iter().next().unwrap()
}

fn cases() -> Vec<(Prepared, Vec<Val>)> {
    let mut out = Vec::new();
    for q in QUERIES {
        if q.contains('$') {
            for pre in PREFIXES {
                out.push((prepare_text(&q.replace('$', &format!("{pre:?}"))).unwrap(), vec![]));
            }
        } else if q.starts_with("query") {
            for pre in PREFIXES {
                out.push((prepare_text(q).unwrap(), vec![Val::Text(pre.into())]));
            }
        } else {
            out.push((prepare_text(q).unwrap(), vec![]));
        }
    }
    out
}

fn agree<B: Backend>(s: Store<B>) {
    populate(&s);
    let limits = Limits { max_rows: usize::MAX, ..Default::default() };
    let mut prefix_reads = 0;
    let mut anti_joins = 0;
    for (p, params) in cases() {
        for strategy in [Strategy::Plain, Strategy::Auto] {
            s.read(|tx| {
                let got = rows(&p, |t| {
                    let r = p.run_tuned(tx, &params, limits, strategy, t).unwrap();
                    if t == TUNINGS[2] {
                        prefix_reads += r.stats.prefix_reads;
                        anti_joins += r.stats.anti_joins;
                    }
                    if t == Tuning::TUPLE_AT_A_TIME {
                        assert_eq!((r.stats.prefix_reads, r.stats.anti_joins), (0, 0));
                    }
                    r
                });
                // A prefix query's rows are exactly the names that start with it.
                if p.print(sophia_query::Profile::Julia).contains("startswith") && got.iter().all(|r| matches!(r[0], Val::Text(_))) {
                    assert!(got.iter().all(|r| matches!(&r[0], Val::Text(t) if params.is_empty() || t.starts_with(match &params[0] { Val::Text(x) => x.as_str(), _ => "" }))));
                }
                Ok(())
            })
            .unwrap();
        }
    }
    assert!(prefix_reads > 0, "the prefix path was taken");
    assert!(anti_joins > 0, "an anti-join was built");
}

/// The prefix path against a direct filter over every name.
fn prefix_reads_exactly_the_range<B: Backend>(s: Store<B>) {
    populate(&s);
    let all = s.read(|tx| tx.names_under("", usize::MAX)).unwrap();
    let p = prepare_text("query q(s::Text)\n select p where name(p, _) && startswith(p, s)\nend").unwrap();
    for pre in PREFIXES {
        let expect: BTreeSet<Vec<Val>> = all.iter().filter(|n| n.path.starts_with(pre)).map(|n| vec![Val::Text(n.path.clone())]).collect();
        let r = s
            .read(|tx| {
                Ok(p.run_tuned(
                    tx,
                    &[Val::Text(pre.into())],
                    Limits { max_rows: usize::MAX, ..Default::default() },
                    Strategy::Auto,
                    Tuning::default(),
                )
                .unwrap())
            })
            .unwrap();
        assert_eq!(r.stats.prefix_reads, 1, "{pre:?}");
        assert_eq!(r.stats.scans, 0, "{pre:?}: no full scan");
        assert_eq!(r.rows.into_iter().collect::<BTreeSet<_>>(), expect, "{pre:?}");
    }
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
            #[test]
            fn tunings_agree() {
                let (s, _d) = store();
                agree(s);
            }
            #[test]
            fn prefix_range() {
                let (s, _d) = store();
                prefix_reads_exactly_the_range(s);
            }
        }
    };
}

suite!(sqlite, |p: &std::path::Path| sophia_store::sqlite::Sqlite::open(p.join("store.sqlite")).unwrap());
suite!(redb, |p: &std::path::Path| sophia_store::redb_store::Redb::open(p.join("store.redb")).unwrap());
#[cfg(feature = "lmdb")]
suite!(lmdb, |p: &std::path::Path| sophia_store::lmdb_store::Lmdb::open_with_map_size(p.join("store.lmdb"), 1 << 30).unwrap());
#[cfg(feature = "duckdb")]
suite!(duckdb, |p: &std::path::Path| sophia_store::duckdb_store::DuckDb::open(p.join("store.duckdb")).unwrap());

/// The SQL translation on SQLite gives the same rows (an independent
/// implementation of the same queries).
#[test]
fn sql_agrees() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("q.sqlite")).unwrap());
    populate(&s);
    let limits = Limits { max_rows: usize::MAX, ..Default::default() };
    for (p, params) in cases() {
        let native: BTreeSet<Vec<Val>> = s
            .read(|tx| Ok(p.run_tuned(tx, &params, limits, Strategy::Auto, Tuning { anti_join_after: 0, prefix_paths: true }).unwrap()))
            .unwrap()
            .rows
            .into_iter()
            .collect();
        let sql: BTreeSet<Vec<Val>> =
            sophia_query::run_sql(s.backend(), &p, &params, limits, Strategy::Auto).unwrap().rows.into_iter().collect();
        assert_eq!(native, sql, "{}\n{params:?}", p.print(sophia_query::Profile::Julia));
    }
}

/// The byte estimate counts base tuples read in the middle of a join (the
/// definitions scanned by the select, the anti-join's set), not only the
/// tuples the select keeps; and the byte budget stops such a run.
#[test]
fn bytes_count_base_reads_inside_joins() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("b.sqlite")).unwrap());
    populate(&s);
    let p = prepare_text("select count(d) where def(d, _) && !refs(_, d)").unwrap();
    let tuning = Tuning { anti_join_after: 0, prefix_paths: true };
    let r = s.read(|tx| Ok(p.run_tuned(tx, &[], Limits::default(), Strategy::Auto, tuning).unwrap())).unwrap();
    // Each scanned `def` tuple alone is a `Vec` header and two values.
    let per = (std::mem::size_of::<Vec<Val>>() + 2 * std::mem::size_of::<Val>()) as u64;
    assert!(r.stats.bytes >= N * per, "{:?}", r.stats);
    let small = Limits { max_bytes: N * per / 2, ..Default::default() };
    let r = s.read(|tx| Ok(p.run_tuned(tx, &[], small, Strategy::Auto, tuning))).unwrap();
    assert!(matches!(&r, Err(sophia_query::Error::Limit(m)) if m.contains("bytes")), "{:?}", r.map(|r| r.stats));
}

/// The budget still stops runs on the new paths: the row cap truncates a
/// prefix read, and the step budget and the cancel flag stop an anti-join.
#[test]
fn limits_hold_on_the_new_paths() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("l.sqlite")).unwrap());
    populate(&s);
    let tuning = Tuning { anti_join_after: 0, prefix_paths: true };
    let prefix = prepare_text("select p where name(p, _) && startswith(p, \"p.q\")").unwrap();
    let r =
        s.read(|tx| Ok(prefix.run_tuned(tx, &[], Limits { max_rows: 5, ..Default::default() }, Strategy::Auto, tuning).unwrap())).unwrap();
    assert_eq!(r.rows.len(), 5);
    assert!(r.truncated);
    assert_eq!(r.stats.prefix_reads, 1);
    let anti = prepare_text("select a, b where def(a, _) && def(b, _) && !refs(a, b)").unwrap();
    let r = s
        .read(|tx| {
            Ok(anti.run_tuned(tx, &[], Limits { max_rows: usize::MAX, max_steps_k: 1, ..Default::default() }, Strategy::Auto, tuning))
        })
        .unwrap();
    assert!(matches!(&r, Err(sophia_query::Error::Limit(m)) if m.contains("steps")), "{:?}", r.map(|r| r.stats));
    let cancel = std::sync::atomic::AtomicBool::new(true);
    let r = s
        .read(|tx| Ok(anti.run_cancellable(tx, &[], Limits { max_rows: usize::MAX, ..Default::default() }, Strategy::Auto, &cancel)))
        .unwrap();
    assert!(matches!(&r, Err(sophia_query::Error::Limit(m)) if m.contains("cancelled")), "{:?}", r.map(|r| r.stats));
}
