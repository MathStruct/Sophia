//! Micro-benchmarks B-01, B-02 and B-04 (vault/Plans/Benchmark Plan.md).

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use sophia_bench::{Shape, closure, generate, ingest};
use sophia_hash::Digest;
use sophia_store::Store;
use std::hint::black_box;

/// B-01: BLAKE3 throughput by input size.
fn b01_hash(c: &mut Criterion) {
    let mut g = c.benchmark_group("B-01 blake3");
    for size in [32usize, 64, 256, 1024, 4096, 65536] {
        let data = vec![7u8; size];
        g.throughput(Throughput::Bytes(size as u64));
        g.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, d| b.iter(|| Digest::of(black_box(d))));
    }
    g.finish();
}

/// B-02: normalise (encode + order + hash) definitions of typical size.
fn b02_normalise(c: &mut Criterion) {
    let program = generate(1, 1000, Shape { recursive_fraction: 0.0, ..Shape::default() });
    // Make each definition standalone: drafts become fixed refs.
    let defs: Vec<_> = program
        .into_iter()
        .map(|(_, d)| {
            d.map_terms(&mut |t| match t {
                sophialang_core::Term::Draft(j) => sophialang_core::Term::Ref(Digest::of(&j.to_le_bytes())),
                t => t,
            })
        })
        .collect();
    let mut g = c.benchmark_group("B-02 normalise");
    g.throughput(Throughput::Elements(defs.len() as u64));
    g.bench_function("1000 definitions, boxed terms", |b| {
        b.iter(|| {
            for d in &defs {
                black_box(sophialang_core::normalise_one(d).unwrap());
            }
        })
    });
    g.finish();
}

/// B-04: point lookups and closure walks, per backend, on a 10k store.
fn b04_lookup(c: &mut Criterion) {
    let program = generate(2, 10_000, Shape::default());
    let dir = tempfile::tempdir().unwrap();
    let sqlite = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("b.sqlite")).unwrap());
    let redb = Store::new(sophia_store::redb_store::Redb::open(dir.path().join("b.redb")).unwrap());
    let records = ingest(&sqlite, &program, 1000);
    ingest(&redb, &program, 1000);
    let probe: Vec<Digest> = records.iter().step_by(97).map(|r| r.id).collect();
    let names: Vec<String> = program.iter().step_by(97).map(|(n, _)| n.clone()).collect();
    let root = records.last().unwrap().id;

    let mut g = c.benchmark_group("B-04 lookup");
    macro_rules! per_backend {
        ($label:literal, $s:expr) => {
            g.bench_function(concat!($label, "/record by id"), |b| {
                b.iter(|| $s.read(|tx| { for d in &probe { black_box(tx.record(d)?); } Ok(()) }).unwrap())
            });
            g.bench_function(concat!($label, "/name"), |b| {
                b.iter(|| $s.read(|tx| { for n in &names { black_box(tx.name(n)?); } Ok(()) }).unwrap())
            });
            g.bench_function(concat!($label, "/dependants"), |b| {
                b.iter(|| $s.read(|tx| { for d in &probe { black_box(tx.dependants(d)?); } Ok(()) }).unwrap())
            });
            g.bench_function(concat!($label, "/closure of last"), |b| {
                b.iter(|| $s.read(|tx| closure(tx, &root)).unwrap())
            });
        };
    }
    per_backend!("sqlite", sqlite);
    per_backend!("redb", redb);
    g.finish();
}

criterion_group!(benches, b01_hash, b02_normalise, b04_lookup);
criterion_main!(benches);
