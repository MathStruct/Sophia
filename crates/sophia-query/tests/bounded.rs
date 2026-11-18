//! The hard memory bound (B-017, SR-108): a query run in the worker process
//! under `bounded::run_bounded` fails with `MemoryBound` when it needs more
//! memory than the bound, on every backend, with the executor's own byte
//! estimate switched off (so the bound comes from outside the executor); and
//! a query within the bound gives the rows of an in-process run.
#![cfg(target_os = "linux")]

use std::path::Path;
use std::time::Duration;

use sophia_hash::{Digest, Record, Status};
use sophia_query::bounded::{Bound, StoreFile, run_bounded};
use sophia_query::{Error, Limits, Strategy, prepare_text};
use sophia_store::{Backend, Store, Visibility};

const WORKER: &str = env!("CARGO_BIN_EXE_sophia-query-worker");
const N: u64 = 700;
const MIB: u64 = 1 << 20;
const BOUND: u64 = 64 * MIB;
/// The first `CHAIN` definitions form a chain.
const CHAIN: u64 = 100;

fn id(i: u64) -> Digest {
    Digest::of(&i.to_le_bytes())
}

/// `N` definitions, each named `d.ni`; the first `CHAIN` form a chain (`i`
/// refers to `i + 1`).
fn populate<B: Backend>(s: &Store<B>) {
    let records: Vec<Record> = (0..N)
        .map(|i| Record {
            id: id(i),
            lang: "test".into(),
            normaliser: "test@0".into(),
            kind: "method".into(),
            component: id(i),
            index: 0,
            component_size: 1,
            bytes: i.to_le_bytes().to_vec(),
            display: Vec::new(),
            status: Status::Complete,
            refs: if i + 1 < CHAIN { vec![id(i + 1)] } else { Vec::new() },
            nominal_refs: Vec::new(),
        })
        .collect();
    let names: Vec<(String, usize, Visibility)> = (0..N as usize).map(|i| (format!("d.n{i}"), i, Visibility::Public)).collect();
    s.add(&records, &names).unwrap();
}

/// No logical limit or byte estimate stops the run before the bound does.
fn unlimited() -> Limits {
    Limits { max_rows: 10, max_steps_k: u64::MAX, max_derived: u64::MAX, max_bytes: u64::MAX, deadline: Some(Duration::from_secs(120)) }
}

/// N² derived pairs: 490 000 tuples here. Measured once with N = 1 500
/// (2026-11-18, a scratch test, not kept because it is slow): in process the
/// executor estimates 936 MB held for 2.25 M pairs; under a 6 GiB bound the
/// worker answers (1 500, 13 s in a debug build), under 128 MiB it stops with
/// `MemoryBound` (1.4 s).
const HEAVY: &str = "query q()\n pair(a, b) = def(a, _) && def(b, _)\n select count(a) where pair(a, b)\nend";
/// The transitive closure of the chain from `d.0`, under the row cap.
const LIGHT: &str = "query q()\n reach(a) = name(\"d.n0\", a)\n reach(b) = reach(a) && refs(a, b)\n select count(b) where reach(b)\nend";

fn check<B: Backend>(make: impl Fn(&Path) -> B, file: impl Fn(&Path) -> StoreFile) {
    let dir = tempfile::tempdir().unwrap();
    let in_process = {
        let s = Store::new(make(dir.path()));
        populate(&s);
        let light = prepare_text(LIGHT).unwrap();
        s.backend().read(|tx| Ok(light.run(tx, &[], unlimited(), Strategy::Auto).unwrap())).unwrap()
        // The store is closed here: redb and DuckDB lock their file.
    };
    let store = file(dir.path());

    let heavy = prepare_text(HEAVY).unwrap();
    let out = run_bounded(&Bound::new(WORKER, BOUND), &store, &heavy, &[], unlimited(), Strategy::Auto);
    assert!(matches!(out, Err(Error::MemoryBound { limit }) if limit == BOUND), "{out:?}");

    let light = prepare_text(LIGHT).unwrap();
    let rows = run_bounded(&Bound::new(WORKER, BOUND), &store, &light, &[], unlimited(), Strategy::Auto).unwrap();
    assert_eq!(
        (rows.columns, rows.sorts, rows.rows, rows.truncated),
        (in_process.columns, in_process.sorts, in_process.rows, in_process.truncated)
    );
}

#[test]
fn sqlite() {
    check(|p| sophia_store::sqlite::Sqlite::open(p.join("s.sqlite")).unwrap(), |p| StoreFile::Sqlite(p.join("s.sqlite")));
}

#[test]
fn redb() {
    check(|p| sophia_store::redb_store::Redb::open(p.join("s.redb")).unwrap(), |p| StoreFile::Redb(p.join("s.redb")));
}

#[cfg(feature = "lmdb")]
#[test]
fn lmdb() {
    check(
        |p| sophia_store::lmdb_store::Lmdb::open_with_map_size(p.join("s.lmdb"), 1 << 30).unwrap(),
        |p| StoreFile::Lmdb(p.join("s.lmdb")),
    );
}

#[cfg(feature = "duckdb")]
#[test]
fn duckdb() {
    check(|p| sophia_store::duckdb_store::DuckDb::open(p.join("s.duckdb")).unwrap(), |p| StoreFile::DuckDb(p.join("s.duckdb")));
}

/// A SQLite store stays readable by the worker while the caller holds it
/// open (WAL), and a query error comes back as its own variant.
#[test]
fn sqlite_while_open_and_errors() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("s.sqlite")).unwrap());
    populate(&s);
    let store = StoreFile::Sqlite(dir.path().join("s.sqlite"));
    let light = prepare_text(LIGHT).unwrap();
    let rows = run_bounded(&Bound::new(WORKER, BOUND), &store, &light, &[], unlimited(), Strategy::Auto).unwrap();
    assert_eq!(rows.rows.len(), 1);

    let limits = Limits { max_derived: 10, ..unlimited() };
    let out = run_bounded(&Bound::new(WORKER, BOUND), &store, &light, &[], limits, Strategy::Auto);
    assert!(matches!(out, Err(Error::Limit(_))), "{out:?}");

    let missing = StoreFile::Sqlite(dir.path().join("missing.sqlite"));
    let out = run_bounded(&Bound::new(WORKER, BOUND), &missing, &light, &[], unlimited(), Strategy::Auto);
    assert!(matches!(out, Err(Error::Store(_))), "{out:?}");

    let out = run_bounded(&Bound::new(dir.path().join("no-worker"), BOUND), &store, &light, &[], unlimited(), Strategy::Auto);
    assert!(matches!(out, Err(Error::Worker(_))), "{out:?}");
}

/// Found by Codex's review (2026-11-18), each failing before its fix:
/// a request larger than the bound (here an 8 MiB parameter) came back as
/// `Worker`, not `MemoryBound`; a store path that is not UTF-8 opened another
/// path; a deadline too far to add to `Instant::now()` panicked in the caller.
#[test]
fn review_findings() {
    use std::os::unix::ffi::OsStringExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(std::ffi::OsString::from_vec(b"store-\xff.sqlite".to_vec()));
    let s = Store::new(sophia_store::sqlite::Sqlite::open(&path).unwrap());
    populate(&s);
    let store = StoreFile::Sqlite(path);
    let light = prepare_text(LIGHT).unwrap();
    let rows = run_bounded(&Bound::new(WORKER, BOUND), &store, &light, &[], unlimited(), Strategy::Auto).unwrap();
    assert_eq!(rows.rows.len(), 1);

    let forever = Limits { deadline: Some(Duration::MAX), ..unlimited() };
    let rows = run_bounded(&Bound::new(WORKER, BOUND), &store, &light, &[], forever, Strategy::Auto).unwrap();
    assert_eq!(rows.rows.len(), 1);

    let prefix = prepare_text("query q(s::Text)\n select a where name(a, _) && startswith(a, s)\nend").unwrap();
    let big = [sophia_query::Val::Text("x".repeat(8 << 20))];
    let out = run_bounded(&Bound::new(WORKER, 4 * MIB), &store, &prefix, &big, unlimited(), Strategy::Auto);
    assert!(matches!(out, Err(Error::MemoryBound { .. })), "{out:?}");
}
