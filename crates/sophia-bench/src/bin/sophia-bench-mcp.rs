//! B-11 through the agent server (B-065): how long a query takes when an
//! agent asks for it through `sophia-mcp`, by backend and by the form it is
//! asked in, split into the layers it passes through. Prints Markdown for
//! vault/Plans/Benchmark Results.md. See `sophia-bench-mcp.md`.
//!
//! Usage: sophia-bench-mcp [--size N] [--backends sqlite,redb,lmdb,duckdb] [--runs R]
//!        [--server PATH] [--worker PATH] [--dir DIR]
//!
//! The server and the worker are `sophia-mcp`'s binaries; by default they are
//! looked for beside this one, so build them first:
//! `cargo build --release -p sophia-mcp --all-features`.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use sophia_bench::{Rng, Shape, generate, ingest};
use sophia_interface::{Interface, Key};
use sophia_mcp::{AnyBackend, Config, Kind, SophiaServer};
use sophia_query::bounded::{Bound, StoreFile, run_bounded};
use sophia_query::{Limits, Prepared, Strategy, Val, prepare_text};
use sophia_store::{Backend, Store};

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

/// The B-17 workloads (`sophia-bench-report`), stored under `bench.<slug>`.
/// The first three take a definition's id (one call per probe).
const WORKLOADS: [(&str, &str, &str); 6] = [
    ("direct dependants", "direct", "query direct(t::Id)\n select a where refs(a, t)\nend"),
    (
        "transitive dependants",
        "dependants",
        "query dependants(t::Id)\n dep(a, b) = refs(a, b)\n dep(a, c) = refs(a, b) && dep(b, c)\n select a where dep(a, t)\nend",
    ),
    (
        "transitive dependencies",
        "dependencies",
        "query dependencies(t::Id)\n dep(a, b) = refs(a, b)\n dep(a, c) = dep(a, b) && refs(b, c)\n select c where dep(t, c)\nend",
    ),
    ("count by kind", "kinds", "select k, count(d) where def(d, k)"),
    ("unreferenced definitions", "unreferenced", "select count(d) where def(d, _) && !refs(_, d)"),
    ("names by prefix", "prefix", "select p where name(p, _) && startswith(p, \"gen.m00001\")"),
];

/// A query that reads one empty range: the worker's fixed cost.
const EMPTY: &str = "select p where name(p, _) && startswith(p, \"zzz\")";

const PROBES: usize = 20;

fn ms(d: Duration) -> String {
    let m = d.as_secs_f64() * 1e3;
    if m < 1.0 {
        format!("{m:.3}")
    } else if m < 100.0 {
        format!("{m:.2}")
    } else {
        format!("{m:.0}")
    }
}

/// Median and 90th percentile.
fn stats(mut v: Vec<Duration>) -> (Duration, Duration) {
    v.sort();
    (v[v.len() / 2], v[(v.len() * 9 / 10).min(v.len() - 1)])
}

fn cell(v: Vec<Duration>) -> String {
    let (p50, p90) = stats(v);
    format!("{} ({})", ms(p50), ms(p90))
}

/// The 1, 5 and 15 minute load averages: the bench is only meaningful on an
/// otherwise idle machine.
fn load() -> String {
    std::fs::read_to_string("/proc/loadavg").map(|l| l.split_whitespace().take(3).collect::<Vec<_>>().join(" ")).unwrap_or_default()
}

fn timed<T>(f: impl FnOnce() -> T) -> (Duration, T) {
    let t = Instant::now();
    let r = f();
    (t.elapsed(), r)
}

/// The store path of `kind` in `dir`.
fn path_of(kind: Kind, dir: &Path) -> PathBuf {
    dir.join(format!("bench.{}", kind.as_str()))
}

/// Builds the store: the synthetic program, the six workloads stored as
/// queries (so `run` finds them), the built-in rendering profiles.
fn build(kind: Kind, path: &Path, program: &[(String, sophialang_core::Definition)]) -> Vec<sophia_hash::Digest> {
    fn fill<B: Backend + Sync>(b: B, program: &[(String, sophialang_core::Definition)]) -> Vec<sophia_hash::Digest> {
        let i = Interface::open(b, "bench").unwrap();
        let records = ingest(i.store(), program, 1000);
        for (_, slug, text) in WORKLOADS {
            i.create_text(&Key::ns("bench"), slug, text).unwrap();
        }
        records.iter().map(|r| r.id).collect()
    }
    match kind {
        Kind::Sqlite => fill(sophia_store::sqlite::Sqlite::open(path).unwrap(), program),
        Kind::Redb => fill(sophia_store::redb_store::Redb::open(path).unwrap(), program),
        Kind::Lmdb => fill(sophia_store::lmdb_store::Lmdb::open(path).unwrap(), program),
        Kind::DuckDb => fill(sophia_store::duckdb_store::DuckDb::open(path).unwrap(), program),
    }
}

/// A raw JSON-RPC client of the server on stdio (as in `sophia-mcp`'s tests).
struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next: u64,
}

impl Client {
    fn start(server: &Path, worker: &Path, kind: Kind, store: &Path) -> Client {
        let mut child = Command::new(server)
            .arg("--store")
            .arg(store)
            .args(["--backend", kind.as_str()])
            .arg("--worker")
            .arg(worker)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot start {}: {e}", server.display()));
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut c = Client { child, stdin, stdout, next: 1 };
        let init = c.request(
            "initialize",
            json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "bench", "version": "0" } }),
        );
        assert!(init["result"]["capabilities"]["tools"].is_object(), "{init}");
        writeln!(c.stdin, "{}", json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).unwrap();
        c
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        writeln!(self.stdin, "{}", json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })).unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        loop {
            line.clear();
            assert!(self.stdout.read_line(&mut line).unwrap() > 0, "the server closed stdout");
            let v: Value = serde_json::from_str(&line).unwrap();
            if v["id"] == json!(id) {
                return v;
            }
        }
    }

    /// Calls a tool; returns its parsed answer, panicking on a tool error.
    fn call(&mut self, tool: &str, args: Value) -> Value {
        let r = self.request("tools/call", json!({ "name": tool, "arguments": args }));
        let text = r["result"]["content"][0]["text"].as_str().unwrap_or_else(|| panic!("{tool}: {r}"));
        assert_ne!(r["result"]["isError"], json!(true), "{tool}: {text}");
        serde_json::from_str(text).unwrap_or(Value::Null)
    }

    fn stop(mut self) {
        drop(self.stdin);
        let _ = self.child.wait();
    }
}

fn rows_of(v: &Value) -> usize {
    v["rows"].as_array().map_or(0, |r| r.len())
}

/// Prints the fixed-cost row and returns the workload rows.
fn measure(kind: Kind, path: &Path, ids: &[sophia_hash::Digest], server: &Path, worker: &Path, runs: usize) -> Vec<String> {
    let mut rng = Rng::new(0x5150);
    let probes: Vec<sophia_hash::Digest> = (0..PROBES).map(|_| ids[rng.below(ids.len() as u64) as usize]).collect();
    let config = {
        let mut c = Config::new(path, worker);
        c.backend = kind;
        c
    };
    // The server's limits: 1 000 rows, 10 s.
    let limits: Limits = config.limits;
    let bound = Bound::new(worker, config.max_memory);
    let file: StoreFile = kind.store_file(path);
    let name = kind.as_str();

    // Fixed costs.
    let (open_t, ()) = {
        let v: Vec<Duration> = (0..runs.max(5)).map(|_| timed(|| drop(AnyBackend::open(kind, path, false).unwrap())).0).collect();
        (stats(v).0, ())
    };
    let local = Store::new(AnyBackend::open(kind, path, false).unwrap());
    let prep: Vec<Duration> = (0..runs * PROBES).map(|_| timed(|| prepare_text(WORKLOADS[1].2).unwrap()).0).collect();
    let empty = prepare_text(EMPTY).unwrap();
    let empty_local: Vec<Duration> =
        (0..runs * PROBES).map(|_| timed(|| local.read(|tx| Ok(empty.run(tx, &[], limits, Strategy::Auto))).unwrap().unwrap()).0).collect();
    // The worker process alone: started with its bound and an empty
    // request, which it refuses and exits; waited for by its exit, not
    // polled as `run_bounded` does.
    let spawn: Vec<Duration> = (0..runs * PROBES)
        .map(|_| {
            timed(|| {
                let o = Command::new(worker).arg(config.max_memory.to_string()).stdin(Stdio::null()).output().unwrap();
                assert!(o.status.success() && !o.stdout.is_empty(), "the worker's answer to an empty request");
            })
            .0
        })
        .collect();
    let empty_worker: Vec<Duration> =
        (0..runs * PROBES).map(|_| timed(|| run_bounded(&bound, &file, &empty, &[], limits, Strategy::Auto).unwrap()).0).collect();
    let starts: Vec<Duration> = (0..runs.max(5))
        .map(|_| {
            let (t, c) = timed(|| Client::start(server, worker, kind, path));
            c.stop();
            t
        })
        .collect();
    // In-process layers first: LMDB lets one process open an environment
    // only once, so `local` is closed before the server's own open below.
    let prepared: Vec<Prepared> = WORKLOADS.iter().map(|(_, _, t)| prepare_text(t).unwrap()).collect();
    let calls = |p: &Prepared| -> Vec<(Vec<Val>, Value)> {
        if p.query.params.is_empty() {
            vec![(vec![], json!([]))]
        } else {
            probes.iter().map(|d| (vec![Val::Id(*d)], json!([d.to_hex()]))).collect()
        }
    };
    let samples = |p: &Prepared| if p.query.params.is_empty() { runs.max(5) } else { runs };
    let mut first = Vec::new();
    for ((label, _, _), p) in WORKLOADS.iter().zip(&prepared) {
        let (mut native, mut sql, mut work) = (vec![], vec![], vec![]);
        let mut rows = Vec::new();
        for _ in 0..samples(p) {
            rows.clear();
            for (a, _) in calls(p) {
                let (t, out) = timed(|| local.read(|tx| Ok(p.run(tx, &a, limits, Strategy::Auto))).unwrap().unwrap());
                native.push(t);
                rows.push(out.rows.len());
                if let AnyBackend::Sqlite(db) = local.backend() {
                    sql.push(timed(|| sophia_query::run_sql(db, p, &a, limits, Strategy::Auto).unwrap()).0);
                }
                let (t, out) = timed(|| run_bounded(&bound, &file, p, &a, limits, Strategy::Auto).unwrap());
                assert_eq!(out.rows.len(), *rows.last().unwrap(), "{name} {label}: the worker's rows");
                work.push(t);
            }
        }
        first.push((native, sql, work, rows.clone()));
    }
    drop(local);

    let server_local = SophiaServer::open(config.clone()).unwrap();
    let mut client = Client::start(server, worker, kind, path);
    let list: Vec<Duration> = (0..runs * PROBES).map(|_| timed(|| client.call("list", json!({ "namespace": "bench" }))).0).collect();
    let empty_mcp: Vec<Duration> = (0..runs * PROBES).map(|_| timed(|| client.call("query", json!({ "text": EMPTY }))).0).collect();
    println!(
        "| {name} | {} | {} | {} | {} | {} | {} | {} | {} |",
        ms(open_t),
        cell(prep),
        cell(empty_local),
        cell(spawn),
        cell(empty_worker),
        cell(list),
        cell(empty_mcp),
        cell(starts)
    );

    let mut lines = Vec::new();
    for (((label, slug, text), p), (native, sql, work, rows)) in WORKLOADS.iter().zip(&prepared).zip(first) {
        let stored = format!("bench.{slug}");
        let (mut srv, mut q, mut r) = (vec![], vec![], vec![]);
        for _ in 0..samples(p) {
            for ((_, ja), want) in calls(p).into_iter().zip(&rows) {
                let ja: Vec<Value> = ja.as_array().unwrap().clone();
                let (t, out) = timed(|| server_local.run_stored(&stored, &ja, None).unwrap());
                assert_eq!(rows_of(&out), *want, "{name} {label}: the server's rows");
                srv.push(t);
                let (t, out) = timed(|| client.call("run", json!({ "name": stored, "params": ja })));
                assert_eq!(rows_of(&out), *want, "{name} {label}: run's rows");
                r.push(t);
                if p.query.params.is_empty() {
                    let (t, out) = timed(|| client.call("query", json!({ "text": text })));
                    assert_eq!(rows_of(&out), *want, "{name} {label}: query's rows");
                    q.push(t);
                }
            }
        }
        let opt = |v: Vec<Duration>| if v.is_empty() { "—".to_string() } else { cell(v) };
        let shown = rows.last().copied().unwrap_or(0);
        lines.push(format!(
            "| {name} | {label} | {shown} | {} | {} | {} | {} | {} | {} |",
            cell(native),
            opt(sql),
            cell(work),
            cell(srv),
            opt(q),
            cell(r)
        ));
    }
    client.stop();
    lines
}

fn main() {
    let n: usize = arg("--size").map(|s| s.parse().unwrap()).unwrap_or(10_000);
    let runs: usize = arg("--runs").map(|s| s.parse().unwrap()).unwrap_or(5);
    let backends: Vec<Kind> = arg("--backends")
        .unwrap_or_else(|| "sqlite,redb,lmdb,duckdb".into())
        .split(',')
        .map(|b| Kind::parse(b).unwrap_or_else(|| panic!("unknown backend {b}")))
        .collect();
    let here = std::env::current_exe().unwrap();
    let server = arg("--server").map(PathBuf::from).unwrap_or_else(|| here.with_file_name("sophia-mcp"));
    let worker = arg("--worker").map(PathBuf::from).unwrap_or_else(|| here.with_file_name("sophia-mcp-worker"));
    for b in [&server, &worker] {
        assert!(b.exists(), "{} is missing: cargo build --release -p sophia-mcp --all-features", b.display());
    }
    // Not /tmp (a RAM disk on the author's machine): the corpora cache.
    let base = arg("--dir").map(PathBuf::from).unwrap_or_else(|| {
        std::env::var_os("SOPHIA_CORPORA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join(".cache/sophia-corpora"))
    });
    std::fs::create_dir_all(&base).unwrap();
    let dir = tempfile::tempdir_in(&base).unwrap();

    let program = generate(42, n, Shape::default());
    println!("#### B-11 through `sophia-mcp` (B-065), {n} definitions, {runs} runs\n");
    println!("Fixed costs, median (p90) ms:\n");
    println!(
        "| backend | open read-only | prepare (text) | empty query, in-process | worker process, no query | empty query, worker | MCP `list` | MCP `query`, empty | server start + initialize |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    let mut work = Vec::new();
    for &kind in &backends {
        let path = path_of(kind, dir.path());
        let ids = build(kind, &path, &program);
        // Let the writes of the build reach the disk before timing.
        let _ = Command::new("sync").status();
        std::thread::sleep(Duration::from_secs(2));
        eprintln!("{}: load before {}", kind.as_str(), load());
        work.extend(measure(kind, &path, &ids, &server, &worker, runs));
        eprintln!("{}: load after {}", kind.as_str(), load());
    }
    println!("\nWorkloads, median (p90) ms per call; rows of the last call (capped at the server's 1 000):\n");
    println!(
        "| backend | workload | rows | native, in-process | SQL, in-process | worker (`run_bounded`) | server, no protocol (`run_stored`) | MCP `query` | MCP `run` |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    work.iter().for_each(|l| println!("{l}"));
}
