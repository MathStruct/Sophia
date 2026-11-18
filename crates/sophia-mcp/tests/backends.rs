//! `sophia-mcp` on every backend it was built with (B-064): the read tools
//! and the worker's queries on all four, the `facts` tool, writes on LMDB,
//! and the write flags refused on redb and DuckDB. See `sophia_mcp.md`,
//! "Tests".

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};
use sophia_hash::Digest;
use sophia_interface::{Interface, Key};
use sophia_store::types::{Annotation, Fact, Input, Target};
use sophia_store::{Backend, sqlite::Sqlite};

const SERVER: &str = env!("CARGO_BIN_EXE_sophia-mcp");
const WORKER: &str = env!("CARGO_BIN_EXE_sophia-mcp-worker");

/// The backends this build serves: `(name, store path in dir)`.
fn built(dir: &Path) -> Vec<(&'static str, PathBuf)> {
    let mut v = vec![("sqlite", dir.join("s.sqlite"))];
    if cfg!(feature = "redb") {
        v.push(("redb", dir.join("s.redb")));
    }
    if cfg!(feature = "lmdb") {
        v.push(("lmdb", dir.join("s.lmdb")));
    }
    if cfg!(feature = "duckdb") {
        v.push(("duckdb", dir.join("s.duckdb")));
    }
    v
}

const USERS_OF: &str = "query users_of(t::Id)\n    uses_(a, b) = refs(a, b)\n    uses_(a, c) = refs(a, b) && uses_(b, c)\n    select p where uses_(a, t) && name(p, a)\nend";

/// What `fill` recorded, to check the answers against.
struct Sample {
    add: Digest,
    add_test: Digest,
}

fn content(i: &Interface<impl Backend + Sync>, path: &str) -> Digest {
    match i.store().resolve(path).unwrap().unwrap().target {
        Target::Content(d) => d,
        t => panic!("{path} names {t:?}"),
    }
}

/// The sample program, a stored query, and something recorded about two of
/// its definitions: a test result (current), a typing whose relation has
/// moved on since (stale), a tag and a proof.
fn fill<B: Backend + Sync>(b: B) -> Sample {
    let i = Interface::open(b, "test").unwrap();
    i.load_sample().unwrap();
    i.create_text(&Key::ns("sample"), "users_of", USERS_OF).unwrap();
    let (add, add_test) = (content(&i, "sample.add"), content(&i, "sample.add_test"));
    let s = i.store();
    let plus = s.new_nominal("sophialang", "generic", "sample.plus").unwrap();
    let v0 = s.relation_version(&plus).unwrap();
    s.add_method(&plus, &add).unwrap();
    let v1 = s.relation_version(&plus).unwrap();
    let rel = |version| Input::Relation { nominal: plus, version };
    s.record_facts(&[
        Fact::new("test-result", add_test, b"pass".to_vec(), ("sophia-test", "0"), vec![Input::Content(add_test), rel(v1)], 7),
        Fact::new("typing", add_test, vec![0xff, 0x00], ("sophia-check", "0"), vec![rel(v0)], 3),
    ])
    .unwrap();
    s.annotate_many(&[
        Annotation::new("tag", vec![add], b"arithmetic".to_vec(), None),
        Annotation::new("proof", vec![add, add_test], Vec::new(), Some(("sophia-own", "0"))),
    ])
    .unwrap();
    Sample { add, add_test }
}

fn make(kind: &str, path: &Path) -> Sample {
    match kind {
        "sqlite" => fill(Sqlite::open(path).unwrap()),
        #[cfg(feature = "redb")]
        "redb" => fill(sophia_store::redb_store::Redb::open(path).unwrap()),
        #[cfg(feature = "lmdb")]
        "lmdb" => fill(sophia_store::lmdb_store::Lmdb::open(path).unwrap()),
        #[cfg(feature = "duckdb")]
        "duckdb" => fill(sophia_store::duckdb_store::DuckDb::open(path).unwrap()),
        k => panic!("not built: {k}"),
    }
}

/// The store's data file: what a read-only session must leave unchanged.
fn data_file(kind: &str, path: &Path) -> PathBuf {
    if kind == "lmdb" { path.join("data.mdb") } else { path.to_path_buf() }
}

struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next: u64,
}

fn command(kind: &str, store: &Path, extra: &[&str]) -> Command {
    let mut c = Command::new(SERVER);
    c.arg("--store").arg(store).args(["--backend", kind, "--worker", WORKER]).args(extra);
    c
}

impl Client {
    fn start(kind: &str, store: &Path, extra: &[&str]) -> Client {
        let mut child = command(kind, store, extra).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn().unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut c = Client { child, stdin, stdout, next: 1 };
        let init = c.request(
            "initialize",
            json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } }),
        );
        assert!(init["result"]["capabilities"]["tools"].is_object(), "{kind}: {init}");
        writeln!(c.stdin, "{}", json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).unwrap();
        c
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        writeln!(self.stdin, "{}", json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })).unwrap();
        self.stdin.flush().unwrap();
        loop {
            let mut line = String::new();
            assert!(self.stdout.read_line(&mut line).unwrap() > 0, "the server closed stdout");
            let v: Value = serde_json::from_str(&line).unwrap();
            if v["id"] == json!(id) {
                return v;
            }
        }
    }

    fn call(&mut self, tool: &str, args: Value) -> (bool, String) {
        let r = self.request("tools/call", json!({ "name": tool, "arguments": args }));
        let res = &r["result"];
        assert!(res.is_object(), "{r}");
        (res["isError"] == json!(true), res["content"][0]["text"].as_str().unwrap_or_default().to_string())
    }

    fn ok(&mut self, tool: &str, args: Value) -> Value {
        let (err, text) = self.call(tool, args);
        assert!(!err, "{tool} failed: {text}");
        serde_json::from_str(&text).unwrap()
    }

    fn tools(&mut self) -> Vec<String> {
        let r = self.request("tools/list", json!({}));
        let mut v: Vec<String> = r["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap().into()).collect();
        v.sort();
        v
    }

    /// The first column of a query's rows, as text.
    fn column(&mut self, tool: &str, args: Value) -> Vec<String> {
        let t = self.ok(tool, args);
        t["rows"].as_array().unwrap().iter().map(|r| r[0].as_str().unwrap().to_string()).collect()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn by_kind<'a>(list: &'a Value, kind: &str) -> &'a Value {
    list.as_array().unwrap().iter().find(|x| x["kind"] == kind).unwrap_or_else(|| panic!("no {kind} in {list}"))
}

/// Every read tool, and the worker's queries, on every backend: a read-only
/// server and its worker share the file (both open it read-only), and the
/// session leaves the store's data file as it was.
#[test]
fn every_backend_serves_the_read_tools() {
    let dir = tempfile::tempdir().unwrap();
    for (kind, path) in built(dir.path()) {
        let s = make(kind, &path);
        let before = std::fs::read(data_file(kind, &path)).unwrap();
        {
            let mut c = Client::start(kind, &path, &[]);
            assert_eq!(c.tools(), ["context", "facts", "list", "query", "run", "show"], "{kind}");
            let names = c.column("query", json!({ "text": "query q select p where name(p, d) end" }));
            assert!(names.iter().any(|n| n == "sample.add_test"), "{kind}: {names:?}");
            let users = c.column("run", json!({ "name": "sample.users_of", "params": ["sample.add"] }));
            assert!(users.iter().any(|n| n == "sample.add_test"), "{kind}: {users:?}");
            assert!(c.ok("show", json!({ "path": "sample.add" }))["text"].as_str().unwrap().contains("add(k, b)"), "{kind}");
            let l = c.ok("list", json!({ "namespace": "sample" }));
            assert!(l["children"].as_array().unwrap().iter().any(|n| n["label"] == "add"), "{kind}: {l}");
            let g = c.ok("context", json!({ "path": "sample.add_test" }));
            assert!(g["nodes"].as_array().unwrap().iter().any(|n| n["label"] == "add"), "{kind}: {g}");

            // facts: a test result that is current, a typing that is stale.
            let f = c.ok("facts", json!({ "path": "sample.add_test" }));
            assert_eq!(f["subject"], s.add_test.to_hex(), "{kind}");
            assert_eq!(f["facts"].as_array().unwrap().len(), 2, "{kind}: {f}");
            let t = by_kind(&f["facts"], "test-result");
            assert_eq!(
                (&t["value"], &t["stale"], &t["by"], &t["derived_at"]),
                (&json!("pass"), &json!(false), &json!("sophia-test"), &json!(7))
            );
            assert_eq!(t["inputs"][0], json!({ "content": s.add_test.to_hex() }), "{kind}: {t}");
            let ty = by_kind(&f["facts"], "typing");
            assert_eq!((&ty["value"], &ty["stale"]), (&json!({ "hex": "ff00" }), &json!(true)), "{kind}: {ty}");
            // The proof names both definitions: seen from add_test, `with` is add.
            assert_eq!(by_kind(&f["annotations"], "proof")["with"], json!([s.add.to_hex()]), "{kind}: {f}");
            assert_eq!(f["truncated"], json!(false));
            // By hex id, the same answer.
            assert_eq!(c.ok("facts", json!({ "path": s.add_test.to_hex() })), f, "{kind}");
            // One kind only.
            let only = c.ok("facts", json!({ "path": "sample.add_test", "kind": "test-result" }));
            assert_eq!((only["facts"].as_array().unwrap().len(), only["annotations"].as_array().unwrap().len()), (1, 0), "{kind}");
            // Tags are annotations.
            let a = c.ok("facts", json!({ "path": "sample.add", "kind": "tag" }));
            let tag = by_kind(&a["annotations"], "tag");
            assert_eq!((&tag["payload"], &tag["with"], a["facts"].as_array().unwrap().len()), (&json!("arithmetic"), &json!([]), 0));
            // Nothing recorded, and no such name.
            let none = c.ok("facts", json!({ "path": "sample.users_of" }));
            assert!(none["facts"].as_array().unwrap().is_empty() && none["annotations"].as_array().unwrap().is_empty(), "{kind}: {none}");
            let (err, msg) = c.call("facts", json!({ "path": "sample.nope" }));
            assert!(err && msg.contains("no name"), "{kind}: {msg}");
        }
        assert!(std::fs::read(data_file(kind, &path)).unwrap() == before, "{kind}: a read-only session changed the store");
    }
}

/// redb and DuckDB refuse the write flags at startup (Daniel, 2026-11-18):
/// the worker could not open the file while the server held it for writing.
#[test]
fn locking_backends_refuse_the_write_flags() {
    let dir = tempfile::tempdir().unwrap();
    for (kind, path) in built(dir.path()) {
        if kind == "sqlite" || kind == "lmdb" {
            continue;
        }
        make(kind, &path);
        let before = std::fs::read(&path).unwrap();
        for flag in ["--allow-edits", "--allow-removals"] {
            let out = command(kind, &path, &[flag]).stdin(Stdio::null()).output().unwrap();
            let err = String::from_utf8_lossy(&out.stderr);
            assert_eq!(out.status.code(), Some(1), "{kind} {flag}: {err}");
            assert!(err.contains("served read-only"), "{kind} {flag}: {err}");
        }
        assert!(std::fs::read(&path).unwrap() == before, "{kind}: a refused start changed the store");
    }
}

/// On LMDB the server writes while its worker reads: what `add` stores, the
/// worker's query finds, and `edit` and `delete` work as on SQLite.
#[cfg(feature = "lmdb")]
#[test]
fn lmdb_serves_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.lmdb");
    make("lmdb", &path);
    let mut c = Client::start("lmdb", &path, &["--allow-edits", "--allow-removals"]);
    assert_eq!(c.tools().len(), 11);
    let three = "function three()::Nat\n    Nat.succ(Nat.succ(Nat.succ(Nat.zero)))\nend\n";
    let a = c.ok("add", json!({ "namespace": "sample", "name": "three", "text": three }));
    assert_eq!(a["path"], "sample.three", "{a}");
    let found = c.column("query", json!({ "text": "query q select d where name(\"sample.three\", d) end" }));
    assert_eq!(found.len(), 1, "the worker sees the server's write: {found:?}");
    let s = c.ok("show", json!({ "path": "sample.add" }));
    let w = c.ok(
        "edit",
        json!({ "path": "sample.add", "version": s["version"], "text": s["text"].as_str().unwrap().replace("add(k, b)", "add(b, k)") }),
    );
    assert_ne!(w["version"], s["version"]);
    c.ok("delete", json!({ "path": "sample.three" }));
    let gone = c.column("query", json!({ "text": "query q select d where name(\"sample.three\", d) end" }));
    assert!(gone.is_empty(), "{gone:?}");
}

/// A missing store, in each backend's shape, and an unknown backend.
#[test]
fn missing_stores_and_unknown_backends() {
    let dir = tempfile::tempdir().unwrap();
    for (kind, path) in built(dir.path()) {
        let out = command(kind, &path, &[]).stdin(Stdio::null()).output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{kind}");
        assert!(String::from_utf8_lossy(&out.stderr).contains(&format!("no {kind} store")), "{kind}");
        assert!(!path.exists(), "{kind}: the server created a store");
    }
    // An LMDB store asked for as SQLite: not a file.
    if cfg!(feature = "lmdb") {
        let path = dir.path().join("l.lmdb");
        make("lmdb", &path);
        let out = command("sqlite", &path, &[]).stdin(Stdio::null()).output().unwrap();
        assert_eq!(out.status.code(), Some(1));
    }
    let out = command("postgres", &dir.path().join("x"), &[]).stdin(Stdio::null()).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--backend must be"));
}

/// Codex's review of B-064, 1: the lists inside a fact (`inputs`) and an
/// annotation (`with`) are capped at `max_nodes` too, and say so.
#[test]
fn review_findings_b064_nested_lists_are_capped() {
    use sophia_mcp::{Config, SophiaServer};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.sqlite");
    let s = make("sqlite", &path);
    {
        let i = Interface::open(Sqlite::open(&path).unwrap(), "test").unwrap();
        let q = content(&i, "sample.users_of");
        i.store().annotate(&Annotation::new("equivalence", vec![s.add_test, s.add, q], Vec::new(), None)).unwrap();
    }
    let mut config = Config::new(&path, WORKER);
    config.max_nodes = 1;
    let server = SophiaServer::open(config).unwrap();
    let f = server.facts_about("sample.add_test", Some("test-result")).unwrap();
    assert_eq!(f["facts"][0]["inputs"].as_array().unwrap().len(), 1, "{f}");
    assert_eq!(f["truncated"], json!(true), "{f}");
    let a = server.facts_about("sample.add_test", Some("equivalence")).unwrap();
    assert_eq!(a["annotations"][0]["with"].as_array().unwrap().len(), 1, "{a}");
    assert_eq!(a["truncated"], json!(true), "{a}");
    // Within the cap: nothing is cut.
    let p = server.facts_about("sample.add_test", Some("proof")).unwrap();
    assert_eq!(p["truncated"], json!(false), "{p}");
}

/// Codex's review of B-064, 2: a file that is not a Sophia store (here an
/// empty one) is refused at start, with or without the write flags, and is
/// not turned into a store.
#[test]
fn review_findings_b064_a_non_store_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    for (kind, path) in built(dir.path()) {
        if kind == "lmdb" {
            std::fs::create_dir(&path).unwrap();
            std::fs::write(path.join("data.mdb"), b"").unwrap();
        } else {
            std::fs::write(&path, b"").unwrap();
        }
        let flags: &[&[&str]] = if kind == "sqlite" || kind == "lmdb" { &[&[], &["--allow-edits"]] } else { &[&[]] };
        for extra in flags {
            let out = command(kind, &path, extra).stdin(Stdio::null()).output().unwrap();
            let err = String::from_utf8_lossy(&out.stderr);
            assert_eq!(out.status.code(), Some(1), "{kind} {extra:?}: {err}");
            assert!(err.contains(&format!("not a Sophia {kind} store")), "{kind} {extra:?}: {err}");
            assert_eq!(std::fs::metadata(data_file(kind, &path)).unwrap().len(), 0, "{kind} {extra:?}: the file was written");
        }
    }
}
