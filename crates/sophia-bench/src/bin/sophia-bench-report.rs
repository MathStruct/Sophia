//! Runs the store benchmarks (B-03, B-04, B-05, B-07 on synthetic code) at
//! larger sizes than criterion handles comfortably, and prints a Markdown
//! report for vault/Plans/Benchmark Results.md.
//!
//! Usage: sophia-bench-report [--sizes 1000,10000,100000] [--backends sqlite,redb,lmdb,duckdb] [--seed N]
//!        [--analytics-size N] [--skip-analytics] [--query-size N] [--skip-queries] [--only-queries] [--zero-copy]

use std::time::{Duration, Instant};

use sophia_bench::{Rng, Shape, closure, generate, ingest, median_time};
use sophia_hash::Digest;
use sophia_store::{Backend, Store};
use sophialang_core::Term;

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

/// A flag without a value.
fn flag(name: &str) -> bool {
    std::env::args().any(|a| a == name)
}

fn machine() -> String {
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("model name")).map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string()))
        .unwrap_or_else(|| "unknown CPU".into());
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0);
    let mem = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| s.lines().next().map(|l| l.split_whitespace().nth(1).unwrap_or("0").parse::<u64>().unwrap_or(0) / 1024 / 1024))
        .unwrap_or(0);
    let os = std::fs::read_to_string("/proc/sys/kernel/osrelease").unwrap_or_default();
    format!("{cpu}, {cores} threads, {mem} GiB RAM, Linux {}", os.trim())
}

fn us(d: Duration) -> String {
    format!("{:.1} µs", d.as_secs_f64() * 1e6)
}

struct Row {
    backend: &'static str,
    n: usize,
    ingest: Duration,
    bytes: Option<u64>,
    by_id: Duration,
    by_name: Duration,
    deps: Duration,
    closure: Duration,
    closure_size: usize,
    feed: Duration,
}

fn run<B: Backend>(store: Store<B>, program: &[(String, sophialang_core::Definition)], seed: u64) -> Row {
    let t = Instant::now();
    let records = ingest(&store, program, 1000);
    let ingest_time = t.elapsed();
    let n = program.len();
    let mut rng = Rng::new(seed ^ 0xABCD);
    let probes: Vec<usize> = (0..1000).map(|_| rng.below(n as u64) as usize).collect();
    let ids: Vec<Digest> = probes.iter().map(|&i| records[i].id).collect();
    let names: Vec<&str> = probes.iter().map(|&i| program[i].0.as_str()).collect();
    let per = |d: Duration| d / probes.len() as u32;

    let by_id = per(median_time(5, || {
        store.read(|tx| { for d in &ids { std::hint::black_box(tx.record(d)?); } Ok(()) }).unwrap()
    }));
    let by_name = per(median_time(5, || {
        store.read(|tx| { for p in &names { std::hint::black_box(tx.name(p)?); } Ok(()) }).unwrap()
    }));
    let deps = per(median_time(5, || {
        store.read(|tx| { for d in &ids { std::hint::black_box(tx.dependants(d)?); } Ok(()) }).unwrap()
    }));
    let root = records.last().unwrap().id;
    let mut closure_size = 0;
    let closure_time = median_time(5, || closure_size = store.read(|tx| closure(tx, &root)).unwrap());
    let feed = median_time(3, || {
        let mut at = 0;
        loop {
            let c = store.changes_since(at, 10_000).unwrap();
            at = c.next;
            if !c.more {
                break;
            }
        }
    });
    Row {
        backend: store.backend().name(),
        n,
        ingest: ingest_time,
        bytes: store.backend().size_on_disk(),
        by_id,
        by_name,
        deps,
        closure: closure_time,
        closure_size,
        feed,
    }
}

/// B-11: analytical queries on the SQL backends.
const ANALYTICS: [(&str, &str); 3] = [
    ("count by kind", "SELECT kind, count(*) FROM definitions GROUP BY kind"),
    ("10 most referenced", "SELECT to_id, count(*) AS n FROM refs GROUP BY to_id ORDER BY n DESC LIMIT 10"),
    ("mean size per status", "SELECT status, avg(bytes) FROM definitions GROUP BY status"),
];

fn analytics<B: Backend + sophia_store::query::ReadOnlyQuery>(store: &Store<B>, n: usize) {
    for (label, q) in ANALYTICS {
        let limits = sophia_store::query::QueryLimits { max_rows: 1000, max_steps_k: 10_000_000 };
        let t = median_time(5, || {
            std::hint::black_box(store.backend().query_readonly(q, limits).unwrap());
        });
        println!("| {} | {n} | {label} | {:.2} ms |", store.backend().name(), t.as_secs_f64() * 1e3);
    }
}

/// B-17: query workloads in Sophia's query language (SPEC-007), on every
/// backend with the native executor, and on SQLite also through the SQL
/// translation. Times are medians of 5 runs; per-probe workloads are divided
/// by the number of probes.
const WORKLOADS: [(&str, &str, bool); 6] = [
    ("direct dependants", "query w(t::Id)\n select a where refs(a, t)\nend", true),
    (
        "transitive dependants",
        "query w(t::Id)\n dep(a, b) = refs(a, b)\n dep(a, c) = refs(a, b) && dep(b, c)\n select a where dep(a, t)\nend",
        true,
    ),
    (
        "transitive dependencies",
        "query w(t::Id)\n dep(a, b) = refs(a, b)\n dep(a, c) = dep(a, b) && refs(b, c)\n select c where dep(t, c)\nend",
        true,
    ),
    ("count by kind", "select k, count(d) where def(d, k)", false),
    ("unreferenced definitions", "select count(d) where def(d, _) && !refs(_, d)", false),
    ("names by prefix", "select p where name(p, _) && startswith(p, \"gen.m00001\")", false),
];

type SqlRun<'a> = &'a dyn Fn(&sophia_query::Prepared, &[sophia_query::Val], sophia_query::Strategy) -> sophia_query::Result<sophia_query::Rows>;

fn queries<B: Backend>(store: &Store<B>, records: &[sophia_hash::Record], n: usize, sql: Option<SqlRun>) {
    use sophia_query::{Limits, Strategy, Val, prepare_text};
    let mut rng = Rng::new(0x5150);
    let probes: Vec<Val> = (0..20).map(|_| Val::Id(records[rng.below(records.len() as u64) as usize].id)).collect();
    let limits = Limits { max_rows: usize::MAX, max_steps_k: 50_000, ..Limits::default() };
    let name = store.backend().name();
    for (label, text, per_probe) in WORKLOADS {
        let p = prepare_text(text).unwrap();
        let args: Vec<Vec<Val>> = if per_probe { probes.iter().map(|v| vec![v.clone()]).collect() } else { vec![vec![]] };
        let mut strategies = vec![("native", Strategy::Auto)];
        if p.magic.is_some() {
            strategies.push(("native, no magic sets (1 probe)", Strategy::Plain));
        }
        let mut cells = Vec::new();
        for (exec, strategy) in strategies {
            // Without magic sets a run computes the whole relation whatever
            // the probe, so one probe, once, is enough to show the cost.
            let (args, runs) = if strategy == Strategy::Plain { (&args[..1], 1) } else { (&args[..], 5) };
            let mut rows = 0;
            let mut failed = None;
            let t = median_time(runs, || {
                rows = 0;
                for a in args {
                    match store.read(|tx| Ok(p.run(tx, a, limits, strategy))).unwrap() {
                        Ok(r) => rows += r.rows.len(),
                        Err(e) => failed = Some(e.to_string()),
                    }
                }
            });
            cells.push((exec.to_string(), t / args.len() as u32, rows / args.len(), failed));
        }
        if let Some(run_sql) = sql {
            for (exec, strategy) in [("SQL", Strategy::Auto), ("SQL, no magic sets (1 probe)", Strategy::Plain)] {
                if exec.contains("no magic") && p.magic.is_none() {
                    continue;
                }
                let (args, runs) = if strategy == Strategy::Plain { (&args[..1], 1) } else { (&args[..], 5) };
                let mut rows = 0;
                let mut failed = None;
                let t = median_time(runs, || {
                    rows = 0;
                    for a in args {
                        match run_sql(&p, a, strategy) {
                            Ok(r) => rows += r.rows.len(),
                            Err(e) => failed = Some(e.to_string()),
                        }
                    }
                });
                cells.push((exec.to_string(), t / args.len() as u32, rows / args.len(), failed));
            }
        }
        for (exec, t, rows, failed) in cells {
            let time = match failed {
                Some(e) if e.contains("derived tuples") => "aborted at the derived-tuple limit".to_string(),
                Some(e) if e.contains("limit") || e.contains("interrupt") => "aborted at the step limit".to_string(),
                Some(e) => format!("error: {e}"),
                None => format!("{:.3} ms", t.as_secs_f64() * 1e3),
            };
            println!("| {name} | {n} | {label} | {exec} | {rows} | {time} |");
        }
    }
}

/// The zero-copy question (SR-101): where the time of an LMDB record read
/// goes. 1000 random probes in one read transaction, median of 5 runs.
fn zero_copy(n: usize, seed: u64, shape: Shape) {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(sophia_store::lmdb_store::Lmdb::open(dir.path().join("z.lmdb")).unwrap());
    let records = ingest(&s, &generate(seed, n, shape), 1000);
    let mut rng = Rng::new(seed ^ 0x2C0);
    let ids: Vec<Digest> = (0..1000).map(|_| records[rng.below(records.len() as u64) as usize].id).collect();
    let per = |d: Duration| d.as_secs_f64() * 1e9 / ids.len() as f64;
    let borrowed = per(median_time(5, || {
        s.backend().with_record_bytes(&ids, |b| { std::hint::black_box(b.map(|b| b.len())); }).unwrap()
    }));
    let copied = per(median_time(5, || {
        s.backend().with_record_bytes(&ids, |b| { std::hint::black_box(b.map(<[u8]>::to_vec)); }).unwrap()
    }));
    let walked = per(median_time(5, || {
        // Walk the stored record's framing in place, without allocating.
        s.backend()
            .with_record_bytes(&ids, |b| {
                let mut r = sophia_hash::canonical::Reader::new(b.unwrap());
                let mut k = 0usize;
                while let Ok((_, p)) = r.next() { k += p.len(); }
                std::hint::black_box(k);
            })
            .unwrap()
    }));
    let record = per(median_time(5, || {
        s.read(|tx| { for d in &ids { std::hint::black_box(tx.record(d)?); } Ok(()) }).unwrap()
    }));
    let term = per(median_time(5, || {
        s.read(|tx| {
            for d in &ids {
                let r = tx.record(d)?.unwrap();
                std::hint::black_box(sophialang_core::encode::decode(&r.bytes).unwrap());
            }
            Ok(())
        })
        .unwrap()
    }));
    let mean = records.iter().map(|r| r.bytes.len()).sum::<usize>() / records.len();
    println!("#### Zero-copy breakdown (LMDB, {n} definitions, mean canonical size {mean} B)\n");
    println!("| step | ns per record |");
    println!("|---|---|");
    println!("| get, borrowed from the map (no copy) | {borrowed:.0} |");
    println!("| get + copy into a Vec | {copied:.0} |");
    println!("| get + walk the framing in place (no allocation) | {walked:.0} |");
    println!("| `ReadTx::record` (copy + decode the record) | {record:.0} |");
    println!("| record + decode the core term (heap objects) | {term:.0} |\n");
}

fn percentile(v: &mut [usize], p: f64) -> usize {
    v.sort_unstable();
    v[((v.len() - 1) as f64 * p) as usize]
}

/// The size for analytics: the `--analytics-size` flag, else the smallest size.
fn analytics_size(sizes: &[usize]) -> usize {
    arg("--analytics-size").map(|s| s.parse().unwrap()).unwrap_or_else(|| *sizes.iter().min().unwrap())
}

fn main() {
    let sizes: Vec<usize> = arg("--sizes")
        .unwrap_or_else(|| "1000,10000,100000".into())
        .split(',')
        .map(|s| s.trim().parse().expect("size"))
        .collect();
    let backends: Vec<String> = arg("--backends")
        .unwrap_or_else(|| "sqlite,redb,lmdb,duckdb".into())
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let seed: u64 = arg("--seed").map(|s| s.parse().unwrap()).unwrap_or(42);
    let shape = Shape::default();

    println!("### Run of {} (seed {seed})\n", std::env::var("SOPHIA_BENCH_DATE").unwrap_or_else(|_| "today".into()));
    println!("- Machine: {}", machine());
    println!("- Build: `cargo run --release -p sophia-bench --bin sophia-bench-report -- --sizes {}`", sizes.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(","));
    println!("- Workload: synthetic sophialang ({shape:?}); ingest in batches of 1000 with names; lookups are 1000 random probes, median of 5 runs.\n");

    // B-07 on the synthetic corpus, and B-05's estimate of local-id savings.
    let corpus = generate(seed, *sizes.iter().max().unwrap_or(&1000).min(&100_000), shape);
    let dir = tempfile::tempdir().unwrap();
    let probe_store = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("stats.sqlite")).unwrap());
    let records = ingest(&probe_store, &corpus, 1000);
    let mut sizes_b: Vec<usize> = records.iter().map(|r| r.bytes.len()).collect();
    let mut ref_occ = 0usize;
    let mut refs_per: Vec<usize> = Vec::new();
    for r in &records {
        let d = sophialang_core::encode::decode(&r.bytes).unwrap();
        let mut k = 0;
        d.walk(&mut |t| if matches!(t, Term::Ref(_)) { k += 1 });
        ref_occ += k;
        refs_per.push(r.refs.len());
    }
    let total: usize = sizes_b.iter().sum();
    let mean = total as f64 / records.len() as f64;
    let med = percentile(&mut sizes_b, 0.5);
    let p90 = percentile(&mut sizes_b, 0.9);
    let mean_refs = refs_per.iter().sum::<usize>() as f64 / refs_per.len() as f64;
    let digest_bytes = ref_occ * 32;
    println!("#### B-07 / B-05: canonical sizes (synthetic corpus, {} definitions)\n", records.len());
    println!("| median bytes | p90 bytes | mean bytes | distinct refs per def | 32-byte reference digests in canonical bytes | key overhead (32 B / mean) | savings with 8-byte local ids |");
    println!("|---|---|---|---|---|---|---|");
    println!(
        "| {med} | {p90} | {mean:.0} | {mean_refs:.2} | {:.1} % of bytes | {:.1} % | {:.1} % of bytes |\n",
        100.0 * digest_bytes as f64 / total as f64,
        100.0 * 32.0 / mean,
        100.0 * (ref_occ * 24) as f64 / total as f64
    );

    if !flag("--skip-queries") {
        let n: usize = arg("--query-size").map(|s| s.parse().unwrap()).unwrap_or_else(|| analytics_size(&sizes));
        println!("#### B-17: query workloads (SPEC-007), {n} definitions\n");
        println!("- Per-probe workloads: 20 random probes, time per probe. Native limits: 50M join steps and 5M derived tuples; SQL step limit 50M VM steps.\n");
        println!("| backend | definitions | workload | executor | rows (per probe) | median |");
        println!("|---|---|---|---|---|---|");
        let program = generate(seed, n, shape);
        for which in &backends {
            let dir = tempfile::tempdir().unwrap();
            match which.as_str() {
                "sqlite" => {
                    let s = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("q.sqlite")).unwrap());
                    let records = ingest(&s, &program, 1000);
                    let db = s.backend();
                    let run = |p: &sophia_query::Prepared, a: &[sophia_query::Val], st| {
                        sophia_query::run_sql(db, p, a, sophia_query::Limits { max_rows: usize::MAX, max_steps_k: 50_000, ..Default::default() }, st)
                    };
                    queries(&s, &records, n, Some(&run));
                }
                "redb" => {
                    let s = Store::new(sophia_store::redb_store::Redb::open(dir.path().join("q.redb")).unwrap());
                    let records = ingest(&s, &program, 1000);
                    queries(&s, &records, n, None);
                }
                "lmdb" => {
                    let s = Store::new(sophia_store::lmdb_store::Lmdb::open(dir.path().join("q.lmdb")).unwrap());
                    let records = ingest(&s, &program, 1000);
                    queries(&s, &records, n, None);
                }
                "duckdb" => {
                    let s = Store::new(sophia_store::duckdb_store::DuckDb::open(dir.path().join("q.duckdb")).unwrap());
                    let records = ingest(&s, &program, 1000);
                    queries(&s, &records, n, None);
                }
                other => panic!("unknown backend {other}"),
            }
        }
        println!();
    }
    if flag("--zero-copy") {
        zero_copy(arg("--query-size").map(|s| s.parse().unwrap()).unwrap_or(100_000), seed, shape);
    }
    if flag("--only-queries") {
        return;
    }

    if !flag("--skip-analytics") {
        println!("#### B-11: analytical queries (SQL backends)\n");
        println!("| backend | definitions | query | median |");
        println!("|---|---|---|---|");
        let n = analytics_size(&sizes);
        let program = generate(seed, n, shape);
        for which in backends.iter().filter(|b| *b == "sqlite" || *b == "duckdb") {
            let dir = tempfile::tempdir().unwrap();
            match which.as_str() {
                "sqlite" => {
                    let s = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("a.sqlite")).unwrap());
                    ingest(&s, &program, 1000);
                    analytics(&s, n);
                }
                _ => {
                    let s = Store::new(sophia_store::duckdb_store::DuckDb::open(dir.path().join("a.duckdb")).unwrap());
                    ingest(&s, &program, 1000);
                    analytics(&s, n);
                }
            }
        }
        println!();
    }

    println!("#### B-03 / B-04 / B-05: backends\n");
    println!("| backend | definitions | ingest | ingest rate | size on disk | bytes / def | record by id | name | dependants | closure of last (size) | full change feed |");
    println!("|---|---|---|---|---|---|---|---|---|---|---|");
    for &n in &sizes {
        let program = generate(seed, n, shape);
        for which in backends.iter() {
            let dir = tempfile::tempdir().unwrap();
            let row = match which.as_str() {
                "sqlite" => run(Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("s.sqlite")).unwrap()), &program, seed),
                "redb" => run(Store::new(sophia_store::redb_store::Redb::open(dir.path().join("s.redb")).unwrap()), &program, seed),
                "lmdb" => run(Store::new(sophia_store::lmdb_store::Lmdb::open(dir.path().join("s.lmdb")).unwrap()), &program, seed),
                "duckdb" => run(Store::new(sophia_store::duckdb_store::DuckDb::open(dir.path().join("s.duckdb")).unwrap()), &program, seed),
                other => panic!("unknown backend {other}"),
            };
            println!(
                "| {} | {} | {:.2} s | {:.0} defs/s | {} | {} | {} | {} | {} | {} ({}) | {:.1} ms |",
                row.backend,
                row.n,
                row.ingest.as_secs_f64(),
                row.n as f64 / row.ingest.as_secs_f64(),
                row.bytes.map(|b| format!("{:.1} MiB", b as f64 / 1048576.0)).unwrap_or("?".into()),
                row.bytes.map(|b| format!("{:.0}", b as f64 / row.n as f64)).unwrap_or("?".into()),
                us(row.by_id),
                us(row.by_name),
                us(row.deps),
                us(row.closure),
                row.closure_size,
                row.feed.as_secs_f64() * 1e3,
            );
        }
    }
}
