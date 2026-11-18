//! `sophia-mcp` end to end: the real binary on stdio, spoken to in raw
//! JSON-RPC lines, on a SQLite store holding the sample program (SPEC-004 §9).
//! See `sophia_mcp.md`, "Tests".

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};
use sophia_interface::Interface;
use sophia_store::sqlite::Sqlite;

const SERVER: &str = env!("CARGO_BIN_EXE_sophia-mcp");
const WORKER: &str = env!("CARGO_BIN_EXE_sophia-mcp-worker");

struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next: u64,
}

impl Client {
    fn start(store: &Path, extra: &[&str]) -> Client {
        let mut child = Command::new(SERVER)
            .arg("--store")
            .arg(store)
            .args(["--worker", WORKER])
            .args(extra)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut c = Client { child, stdin, stdout, next: 1 };
        let init = c.request(
            "initialize",
            json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } }),
        );
        assert!(init["result"]["capabilities"]["tools"].is_object(), "{init}");
        c.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        c
    }

    fn send(&mut self, msg: Value) {
        writeln!(self.stdin, "{msg}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let mut line = String::new();
            assert!(self.stdout.read_line(&mut line).unwrap() > 0, "the server closed stdout");
            let v: Value = serde_json::from_str(&line).unwrap();
            if v["id"] == json!(id) {
                return v;
            }
        }
    }

    /// Calls a tool: (is it an error, its text).
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
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn sample_store(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("s.sqlite");
    let i = Interface::open(Sqlite::open(&path).unwrap(), "test").unwrap();
    i.load_sample().unwrap();
    path
}

impl Client {
    fn tools(&mut self) -> Vec<String> {
        let r = self.request("tools/list", json!({}));
        let mut names: Vec<String> =
            r["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
        names.sort();
        names
    }
}

const READ: [&str; 6] = ["context", "facts", "list", "query", "run", "show"];
const WRITE: [&str; 3] = ["add", "edit", "save_query"];
const REMOVE: [&str; 2] = ["delete", "rename"];

fn sorted(parts: &[&[&str]]) -> Vec<String> {
    let mut v: Vec<String> = parts.iter().flat_map(|p| p.iter().map(|s| s.to_string())).collect();
    v.sort();
    v
}

/// Each class of tools is listed only with its flag (SR-157, SR-158).
#[test]
fn tools_are_listed_by_class() {
    let dir = tempfile::tempdir().unwrap();
    let store = sample_store(dir.path());
    assert_eq!(Client::start(&store, &[]).tools(), sorted(&[&READ]));
    assert_eq!(Client::start(&store, &["--allow-edits"]).tools(), sorted(&[&READ, &WRITE]));
    assert_eq!(Client::start(&store, &["--allow-removals"]).tools(), sorted(&[&READ, &REMOVE]));
    assert_eq!(Client::start(&store, &["--allow-edits", "--allow-removals"]).tools(), sorted(&[&READ, &WRITE, &REMOVE]));
}

/// Without the flags, a write tool cannot be called even by name, and the
/// store file is not touched by a whole session.
#[test]
fn a_read_only_server_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let store = sample_store(dir.path());
    let before = std::fs::read(&store).unwrap();
    {
        let mut c = Client::start(&store, &[]);
        let s = c.ok("show", json!({ "path": "sample.add" }));
        for (tool, args) in [
            ("edit", json!({ "path": "sample.add", "version": s["version"], "text": s["text"] })),
            ("add", json!({ "namespace": "sample", "name": "x", "text": "function x()::Nat\n    zero\nend\n" })),
            ("save_query", json!({ "namespace": "sample", "name": "q", "text": "query q select p where name(p, d) end" })),
            ("rename", json!({ "path": "sample.add", "to": "sample.plus" })),
            ("delete", json!({ "path": "sample.add" })),
        ] {
            let r = c.request("tools/call", json!({ "name": tool, "arguments": args }));
            let refused = r["error"].is_object() || r["result"]["isError"] == json!(true);
            assert!(refused, "{tool}: {r}");
        }
        c.ok("query", json!({ "text": "query q select p where name(p, d) end" }));
    }
    assert!(std::fs::read(&store).unwrap() == before, "the store file changed");
    // Removals alone do not allow edits, and edits alone do not allow removals.
    let mut c = Client::start(&store, &["--allow-removals"]);
    let s = c.ok("show", json!({ "path": "sample.add" }));
    let r = c.request(
        "tools/call",
        json!({ "name": "edit", "arguments": { "path": "sample.add", "version": s["version"], "text": s["text"] } }),
    );
    assert!(r["error"].is_object() || r["result"]["isError"] == json!(true), "{r}");
    let mut c = Client::start(&store, &["--allow-edits"]);
    let r = c.request("tools/call", json!({ "name": "delete", "arguments": { "path": "sample.add" } }));
    assert!(r["error"].is_object() || r["result"]["isError"] == json!(true), "{r}");
    c.ok("show", json!({ "path": "sample.add" }));
}

/// `edit`: the version check, diagnostics, and a real change (SPEC-004 §3).
#[test]
fn edits_through_the_server() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::start(&sample_store(dir.path()), &["--allow-edits"]);
    let s = c.ok("show", json!({ "path": "sample.add" }));
    let text = s["text"].as_str().unwrap();
    let version = s["version"].as_u64().unwrap();
    assert!(text.contains("add(k, b)"), "{text}");
    let changed = text.replace("add(k, b)", "add(b, k)");

    // A parse error stores nothing and says where.
    let bad = format!("{text}\nfunction (\n");
    let (err, msg) = c.call("edit", json!({ "path": "sample.add", "version": version, "text": bad }));
    assert!(err, "{msg}");
    let e: Value = serde_json::from_str(&msg).unwrap();
    assert_eq!(e["error"], "diagnostics", "{e}");
    let d = &e["diagnostics"][0];
    assert!(d["line"].as_u64().unwrap() > 1 && d["start"].as_u64().unwrap() >= text.len() as u64, "{e}");
    assert_eq!(c.ok("show", json!({ "path": "sample.add" }))["version"], json!(version), "nothing was stored");

    // A stale version is a conflict.
    let (err, msg) = c.call("edit", json!({ "path": "sample.add", "version": version + 1, "text": changed }));
    let e: Value = serde_json::from_str(&msg).unwrap();
    assert!(err && e["error"] == "conflict" && e["found"] == json!(version), "{msg}");

    // The right version stores the change and moves the name.
    let w = c.ok("edit", json!({ "path": "sample.add", "version": version, "text": changed }));
    assert_ne!(w["version"], json!(version), "{w}");
    assert!(w["targets"].as_array().unwrap().iter().any(|t| t["name"] == "sample.add"), "{w}");
    let after = c.ok("show", json!({ "path": "sample.add" }));
    assert!(after["text"].as_str().unwrap().contains("add(b, k)"), "{after}");
    assert_eq!(after["version"], w["version"]);
    // The old version is now stale.
    let (err, _) = c.call("edit", json!({ "path": "sample.add", "version": version, "text": text }));
    assert!(err);

    // add: a new name in the namespace; a dotted name is refused.
    let a = c
        .ok("add", json!({ "namespace": "sample", "name": "three", "text": "function three()::Nat\n    Nat.succ(Nat.succ(Nat.succ(Nat.zero)))\nend\n" }));
    assert_eq!(a["path"], "sample.three", "{a}");
    assert!(c.ok("list", json!({ "namespace": "sample" }))["children"].as_array().unwrap().iter().any(|n| n["label"] == "three"));
    // The query worker, a separate process, sees what this server wrote.
    let t = c.ok("query", json!({ "text": "query q select d where name(\"sample.three\", d) end" }));
    assert_eq!(t["rows"].as_array().unwrap().len(), 1, "{t}");
    let (err, _) = c.call("add", json!({ "namespace": "sample", "name": "a.b", "text": "function b()::Nat\n    Nat.zero\nend\n" }));
    assert!(err);
}

/// `save_query` and `run`: a stored query with a parameter, called by name.
#[test]
fn stored_queries_run_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let store = sample_store(dir.path());
    let q = "query users_of(t::Id)\n    uses_(a, b) = refs(a, b)\n    uses_(a, c) = refs(a, b) && uses_(b, c)\n    select p where uses_(a, t) && name(p, a)\nend";
    {
        let mut c = Client::start(&store, &["--allow-edits"]);
        // Text with a parameter cannot run as text.
        let (err, msg) = c.call("query", json!({ "text": q }));
        assert!(err && msg.contains("save_query"), "{msg}");
        // save_query takes only queries.
        let (err, _) =
            c.call("save_query", json!({ "namespace": "sample", "name": "f", "text": "function f()::Nat\n    Nat.zero\nend\n" }));
        assert!(err);
        c.ok("save_query", json!({ "namespace": "sample", "name": "users_of", "text": q }));
    }
    // A read-only server runs it.
    let mut c = Client::start(&store, &[]);
    let t = c.ok("run", json!({ "name": "sample.users_of", "params": ["sample.add"] }));
    assert!(t["rows"].as_array().unwrap().iter().any(|r| r[0] == "sample.add_test"), "{t}");
    // By hex id too.
    let id = c.ok("query", json!({ "text": "query q select d where name(\"sample.add\", d) end" }))["rows"][0][0].clone();
    let by_id = c.ok("run", json!({ "name": "sample.users_of", "params": [id] }));
    assert_eq!(by_id["rows"], t["rows"]);
    for params in [json!([]), json!([1]), json!(["sample.nothing"]), json!(["sample.add", "sample.add"])] {
        let (err, msg) = c.call("run", json!({ "name": "sample.users_of", "params": params }));
        assert!(err, "{params}: {msg}");
    }
    let (err, _) = c.call("run", json!({ "name": "sample.nothing" }));
    assert!(err);
}

/// `rename` and `delete` move and remove names; the definitions stay.
#[test]
fn removals_keep_the_definitions() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::start(&sample_store(dir.path()), &["--allow-removals"]);
    let defs = "query q select count(d) where def(d, _) end";
    let before = c.ok("query", json!({ "text": defs }))["rows"].clone();
    let old = c.ok("show", json!({ "path": "sample.add_test" }))["text"].clone();

    let r = c.ok("rename", json!({ "path": "sample.add_test", "to": "sample.check_add" }));
    assert_eq!(r["to"], "sample.check_add", "{r}");
    // The same definition under the new name (the print names it).
    let new = c.ok("show", json!({ "path": "sample.check_add" }))["text"].clone();
    assert_eq!(new.as_str().unwrap(), old.as_str().unwrap().replace("add_test", "check_add"));
    assert!(c.call("show", json!({ "path": "sample.add_test" })).0);

    c.ok("delete", json!({ "path": "sample.check_add" }));
    assert!(c.call("show", json!({ "path": "sample.check_add" })).0);
    assert!(c.call("delete", json!({ "path": "sample.check_add" })).0, "deleting a missing name is an error");
    assert_eq!(c.ok("query", json!({ "text": defs }))["rows"], before, "no definition was removed");
}

#[test]
fn read_tools_on_the_sample() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::start(&sample_store(dir.path()), &[]);

    // query: who uses sample.add, transitively (as in the interface's acceptance test).
    let q = r#"query users
        uses_(a, b) = refs(a, b)
        uses_(a, c) = refs(a, b) && uses_(b, c)
        select a, p where name("sample.add", t) && uses_(a, t) && name(p, a)
    end"#;
    let t = c.ok("query", json!({ "text": q }));
    assert_eq!(t["columns"], json!(["a", "p"]));
    let rows = t["rows"].as_array().unwrap();
    assert!(rows.iter().any(|r| r[1] == "sample.add_test"), "{t}");
    // Every id cell is described.
    for r in rows {
        let id = r[0].as_str().unwrap();
        assert_eq!(t["ids"][id]["kind"], "Definition", "{t}");
    }
    // max_rows caps the answer.
    let one = c.ok("query", json!({ "text": q, "max_rows": 1 }));
    assert!(one["rows"].as_array().unwrap().len() <= 1 && one["truncated"] == json!(rows.len() > 1), "{one}");

    // show: the text and a version token.
    let s = c.ok("show", json!({ "path": "sample.add" }));
    assert!(s["text"].as_str().unwrap().contains("add"), "{s}");
    assert!(s["version"].as_u64().unwrap() > 0, "{s}");

    // list: the namespace holds the name.
    let l = c.ok("list", json!({ "namespace": "sample" }));
    assert!(l["children"].as_array().unwrap().iter().any(|n| n["label"] == "add"), "{l}");
    let root = c.ok("list", json!({}));
    assert!(root["children"].as_array().unwrap().iter().any(|n| n["label"] == "sample"), "{root}");

    // context: sample.add_test uses sample.add.
    let g = c.ok("context", json!({ "path": "sample.add_test", "depth": 3 }));
    let nodes = g["nodes"].as_array().unwrap();
    let edges = g["edges"].as_array().unwrap();
    assert!(nodes.iter().filter(|n| n["kind"] == "Definition").count() >= 2, "{g}");
    assert!(edges.iter().any(|e| e["kind"] == "References"), "{g}");
    for e in edges {
        assert!(nodes.iter().any(|n| n["ref"] == e["from"]) && nodes.iter().any(|n| n["ref"] == e["to"]), "{g}");
    }
}

/// `direction: "in"` finds what uses the name's definition, not the
/// namespaces above the name (Codex's review of B-062).
#[test]
fn context_in_finds_the_users() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::start(&sample_store(dir.path()), &[]);
    for d in ["in", "both"] {
        let g = c.ok("context", json!({ "path": "sample.add", "depth": 1, "direction": d }));
        let nodes = g["nodes"].as_array().unwrap();
        assert!(nodes.iter().any(|n| n["kind"] == "Definition" && n["label"] == "add_test"), "{d}: {g}");
        assert!(nodes.iter().any(|n| n["kind"] == "Name" && n["label"] == "add"), "{d}: the name stays: {g}");
    }
    // `out` at depth 1 still reaches what the definition uses, not only the definition.
    let g = c.ok("context", json!({ "path": "sample.add_test", "depth": 1 }));
    assert!(g["nodes"].as_array().unwrap().iter().any(|n| n["kind"] == "Definition" && n["label"] == "add"), "{g}");
}

#[test]
fn errors_are_tool_errors() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::start(&sample_store(dir.path()), &[]);
    let cases = [
        ("show", json!({ "path": "sample.nothing" })),
        ("query", json!({ "text": "query q select x where nosuch(x) end" })),
        ("query", json!({ "text": "not a query" })),
        ("context", json!({ "path": "sample.add", "depth": 9 })),
        ("context", json!({ "path": "sample.add", "direction": "up" })),
        ("context", json!({ "path": "sample.nothing" })),
    ];
    for (tool, args) in cases {
        let (err, text) = c.call(tool, args.clone());
        assert!(err && !text.is_empty(), "{tool} {args}: {text}");
    }
    // The server is still answering after the errors.
    c.ok("show", json!({ "path": "sample.add" }));
}

/// Queries go through the worker under the memory bound: below about 1 MiB
/// the worker cannot start, so every query fails with the bound's error
/// (B-017), while the tools that read in the server still work.
#[test]
fn queries_run_under_the_memory_bound() {
    let dir = tempfile::tempdir().unwrap();
    let store = sample_store(dir.path());
    let mut c = Client::start(&store, &["--max-memory", "0"]);
    let (err, text) = c.call("query", json!({ "text": "query q select p where name(p, d) end" }));
    assert!(err && text.contains("memory"), "{text}");
    c.ok("show", json!({ "path": "sample.add" }));
}

#[test]
fn a_missing_store_is_refused_at_start() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(SERVER).arg("--store").arg(dir.path().join("none.sqlite")).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("no sqlite store"));
    let out = Command::new(SERVER).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}

/// Codex's review of B-063: a name path to a nominal entity is a valid `Id`
/// parameter; `add` answers with the name the text defined and a version
/// token that `edit` accepts.
#[test]
fn review_findings_b063() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::start(&sample_store(dir.path()), &["--allow-edits"]);

    // A relation (nominal) by name and by hex id gives the same members.
    c.ok(
        "save_query",
        json!({ "namespace": "sample", "name": "members", "text": "query members(n::Id)\n    select d where member(n, d)\nend" }),
    );
    let id = c.ok("query", json!({ "text": "query q select n where name(\"sample.parent\", n) end" }))["rows"][0][0].clone();
    let by_id = c.ok("run", json!({ "name": "sample.members", "params": [id] }));
    assert!(!by_id["rows"].as_array().unwrap().is_empty(), "{by_id}");
    let by_name = c.ok("run", json!({ "name": "sample.members", "params": ["sample.parent"] }));
    assert_eq!(by_name["rows"], by_id["rows"]);

    // add: the version it returns is the one edit needs.
    let a = c.ok("add", json!({ "namespace": "sample", "name": "four", "text": "function four()::Nat\n    Nat.succ(three())\nend\nfunction three()::Nat\n    Nat.succ(Nat.succ(Nat.succ(Nat.zero)))\nend\n" }));
    assert_eq!(a["path"], "sample.four", "{a}");
    assert_eq!(a["version"], c.ok("show", json!({ "path": "sample.four" }))["version"], "{a}");
    let text = c.ok("show", json!({ "path": "sample.four" }))["text"]
        .as_str()
        .unwrap()
        .replace("Nat.succ(three())", "Nat.succ(Nat.succ(three()))");
    c.ok("edit", json!({ "path": "sample.four", "version": a["version"], "text": text }));

    // add: when the text does not define `name`, the answer names what it did define.
    let a = c.ok("add", json!({ "namespace": "sample", "name": "foo", "text": "function bar()::Nat\n    Nat.zero\nend\n" }));
    assert_eq!(a["path"], "sample.bar", "{a}");
    assert_eq!(a["version"], c.ok("show", json!({ "path": "sample.bar" }))["version"], "{a}");
}
