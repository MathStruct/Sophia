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

#[test]
fn tools_are_listed() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::start(&sample_store(dir.path()), &[]);
    let r = c.request("tools/list", json!({}));
    let mut names: Vec<&str> = r["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    names.sort();
    assert_eq!(names, ["context", "list", "query", "show"]);
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
    assert!(String::from_utf8_lossy(&out.stderr).contains("no store file"));
    let out = Command::new(SERVER).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}
