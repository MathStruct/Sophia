//! Mistral's review of B-063 (the permission classes of `sophia-mcp`).
//! The `Client` helper is copied from `tests/stdio.rs`. Each test below
//! comments the finding it looks for and what it expects; none of them
//! repeats a check `stdio.rs` already makes.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
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

    /// Sends a request and returns its id without waiting for the answer
    /// (for sending two calls back to back).
    fn send_request(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next;
        self.next += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        id
    }

    /// Reads answers until every id in `ids` has arrived.
    fn read_until(&mut self, ids: &[u64]) -> Vec<Value> {
        let mut out = Vec::new();
        while out.len() < ids.len() {
            let mut line = String::new();
            assert!(self.stdout.read_line(&mut line).unwrap() > 0, "the server closed stdout");
            let v: Value = serde_json::from_str(&line).unwrap();
            if v["id"].as_u64().is_some_and(|id| ids.contains(&id)) {
                out.push(v);
            }
        }
        out
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

    fn tools(&mut self) -> Vec<String> {
        let r = self.request("tools/list", json!({}));
        let mut names: Vec<String> =
            r["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
        names.sort();
        names
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn sample_store(dir: &Path) -> PathBuf {
    let path = dir.join("s.sqlite");
    let i = Interface::open(Sqlite::open(&path).unwrap(), "test").unwrap();
    i.load_sample().unwrap();
    path
}

/// The store directory, sorted: a read-only session must not even create
/// SQLite sidecar files.
fn dir_listing(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

/// FINDING CHECKED: "without either flag the server must not change the
/// store file at all" — stdio.rs checks only the main file's bytes. This
/// checks the whole directory (no `-wal`/`-shm`/`-journal` from the
/// server's or the worker's open) and the file's mtime, after a session
/// that also ran a `query`, so the worker opened the store too.
/// Expected by the review: pass — `Sqlite::open_read_only` (sqlite.rs) opens with
/// SQLITE_OPEN_READ_ONLY and skips `SCHEMA`, `Interface::open_read_only`
/// skips `ensure_built_ins`, and the worker's `serve` opens read-only too.
#[test]
fn read_only_session_writes_nothing_to_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let store = sample_store(dir.path());
    let before_files = dir_listing(dir.path());
    let before_bytes = std::fs::read(&store).unwrap();
    let before_mtime = std::fs::metadata(&store).unwrap().modified().unwrap();
    {
        let mut c = Client::start(&store, &[]);
        c.ok("show", json!({ "path": "sample.add" }));
        c.ok("list", json!({ "namespace": "sample" }));
        c.ok("context", json!({ "path": "sample.add", "depth": 2 }));
        c.ok("query", json!({ "text": "query q select p where name(p, d) end" }));
    }
    // Run on 2026-11-18, this failed: the session leaves `s.sqlite-shm` and
    // an empty `s.sqlite-wal`. Every SQLite reader of a WAL-mode file creates
    // them (the shared-memory index and the log); `immutable=1` would avoid
    // them but is unsafe while another process writes. Accepted as
    // documented behaviour ([[sophia_mcp]]): only those two may appear, and
    // the log must be empty, so nothing was written.
    let mut allowed = before_files.clone();
    allowed.extend(["s.sqlite-shm".to_string(), "s.sqlite-wal".to_string()]);
    allowed.sort();
    let after = dir_listing(dir.path());
    assert!(after.iter().all(|f| allowed.contains(f)) && before_files.iter().all(|f| after.contains(f)), "{after:?}");
    if let Ok(m) = std::fs::metadata(dir.path().join("s.sqlite-wal")) {
        assert_eq!(m.len(), 0, "the read-only session wrote to the WAL");
    }
    assert_eq!(std::fs::read(&store).unwrap(), before_bytes, "the store file changed");
    assert_eq!(std::fs::metadata(&store).unwrap().modified().unwrap(), before_mtime, "the store file's mtime changed");
}

/// FINDING CHECKED: a disallowed tool must be *not callable*, not just
/// refused — the route is removed at startup (`SophiaServer::open`), so
/// `tools/call` must fail without the tool's method ever running. The
/// observable: the answer must not contain `permit`'s message "started
/// without …", which only `edit_text`/`add_text`/`rename_name`/`delete_name`
/// produce when they run. (stdio.rs only checks that the call is refused.)
/// Expects: pass. If it fails, the call was dispatched to the handler and
/// only the second lock (`permit`) stopped it — the store would still be
/// safe, which the byte check below also asserts.
#[test]
fn a_disallowed_tool_is_not_dispatched() {
    let dir = tempfile::tempdir().unwrap();
    let store = sample_store(dir.path());
    let before = std::fs::read(&store).unwrap();
    let mut c = Client::start(&store, &[]);
    let s = c.ok("show", json!({ "path": "sample.add" }));
    let version = s["version"].as_u64().unwrap();
    for (tool, args) in [
        ("edit", json!({ "path": "sample.add", "version": version, "text": s["text"] })),
        ("add", json!({ "namespace": "sample", "name": "x", "text": "function x()::Nat\n    zero\nend\n" })),
        ("save_query", json!({ "namespace": "sample", "name": "q", "text": "query q select p where name(p, d) end" })),
        ("rename", json!({ "path": "sample.add", "to": "sample.plus" })),
        ("delete", json!({ "path": "sample.add" })),
    ] {
        let r = c.request("tools/call", json!({ "name": tool, "arguments": args }));
        let refused = r["error"].is_object() || r["result"]["isError"] == json!(true);
        assert!(refused, "{tool}: {r}");
        assert!(!r.to_string().contains("started without"), "{tool} was dispatched to its method: {r}");
    }
    assert_eq!(std::fs::read(&store).unwrap(), before, "the store file changed");
}

/// FINDING CHECKED: "can argument parsing turn a flag on by accident?"
/// The flags take no value and match only the exact strings; anything else
/// is an error and the server exits 2 instead of starting. A flag-like
/// token must never be honoured as a flag.
/// Expects: pass — `--allow-edits=true` and a stray value after a flag are
/// "unknown argument" (exit 2), and `--max-memory --allow-removals` fails
/// the u64 parse (exit 2) so `--allow-removals` is consumed as a value,
/// not honoured. The control shows a plain `--allow-edits` does list `edit`.
#[test]
fn flag_like_arguments_are_refused_not_honoured() {
    let dir = tempfile::tempdir().unwrap();
    let store = sample_store(dir.path());
    for bad in ["--allow-edits=true", "--allow-removals=1"] {
        let out = Command::new(SERVER).arg("--store").arg(&store).arg(bad).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{bad} started the server");
        assert!(String::from_utf8_lossy(&out.stderr).contains("unknown argument"), "{bad}: {out:?}");
    }
    // A stray value after a flag is an unknown argument, not a second flag.
    let out = Command::new(SERVER).arg("--store").arg(&store).arg("--allow-edits").arg("yes").output().unwrap();
    assert_eq!(out.status.code(), Some(2), "the server started with a stray value");
    // A flag swallowed as another option's value is not honoured.
    let out = Command::new(SERVER).arg("--store").arg(&store).arg("--max-memory").arg("--allow-removals").output().unwrap();
    assert_eq!(out.status.code(), Some(2), "the server started with --allow-removals as a value");
    // Control: the plain flag works.
    let mut c = Client::start(&store, &["--allow-edits"]);
    assert!(c.tools().contains(&"edit".to_string()));
}

/// FINDING CHECKED: "can `run` reach something other than a bounded read?"
/// `run` loads a stored query by name or id; `Prepared::from_record`
/// (sophia_query.rs) refuses any record whose lang is not the query
/// language, so a normal definition cannot be executed as a query.
/// Expects: pass — `run` on `sample.add` (a function) is an error and the
/// store is untouched.
#[test]
fn run_cannot_execute_a_non_query_definition() {
    let dir = tempfile::tempdir().unwrap();
    let store = sample_store(dir.path());
    let before = std::fs::read(&store).unwrap();
    let mut c = Client::start(&store, &[]);
    let (err, msg) = c.call("run", json!({ "name": "sample.add" }));
    assert!(err, "run executed a non-query definition: {msg}");
    assert_eq!(std::fs::read(&store).unwrap(), before, "the store file changed");
}

/// FINDING CHECKED: "`delete` removes a name only". The MCP tools build
/// `Key::name(path)` (always the `name:` prefix), so the interface's
/// namespace-deletion branch is unreachable from the server: deleting a
/// namespace path must be an error and the namespace must survive.
/// Expects: pass.
#[test]
fn delete_on_a_namespace_path_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let store = sample_store(dir.path());
    let mut c = Client::start(&store, &["--allow-removals"]);
    let (err, msg) = c.call("delete", json!({ "path": "sample" }));
    assert!(err, "delete removed a namespace: {msg}");
    let (err, _) = c.call("delete", json!({ "path": "" }));
    assert!(err);
    let l = c.ok("list", json!({ "namespace": "sample" }));
    assert!(l["children"].as_array().unwrap().iter().any(|n| n["label"] == "add"), "{l}");
    c.ok("show", json!({ "path": "sample.add" }));
}

/// FINDING CHECKED: race on the version check — two `edit` calls quoting
/// the same version, sent back to back. Exactly one may move the name; the
/// other must be a conflict. The check is `Store::update`'s compare-and-
/// swap inside one write transaction (logic.rs:257-263), not just the
/// pre-check in `write_text`.
/// Expects: pass — exactly one success, one error; afterwards `show` prints
/// one of the two texts. A defect would be both succeeding (lost update).
#[test]
fn concurrent_edits_with_one_version_one_wins() {
    let dir = tempfile::tempdir().unwrap();
    let store = sample_store(dir.path());
    let mut c = Client::start(&store, &["--allow-edits"]);
    let s = c.ok("show", json!({ "path": "sample.add" }));
    let version = s["version"].as_u64().unwrap();
    let text = s["text"].as_str().unwrap().to_string();
    assert!(text.contains("add(k, b)"), "{text}");
    let t1 = text.replace("add(k, b)", "add(b, k)");
    let t2 = text.replace("add(k, b)", "add(c, k)");
    let id1 =
        c.send_request("tools/call", json!({ "name": "edit", "arguments": { "path": "sample.add", "version": version, "text": t1 } }));
    let id2 =
        c.send_request("tools/call", json!({ "name": "edit", "arguments": { "path": "sample.add", "version": version, "text": t2 } }));
    let rs = c.read_until(&[id1, id2]);
    let ok = rs.iter().filter(|r| r["error"].is_null() && r["result"]["isError"] != json!(true)).count();
    let err = rs.iter().filter(|r| r["error"].is_object() || r["result"]["isError"] == json!(true)).count();
    assert_eq!((ok, err), (1, 1), "two edits with one version: {rs:?}");
    let after = c.ok("show", json!({ "path": "sample.add" }));
    let at = after["text"].as_str().unwrap();
    assert!(at.contains("add(b, k)") || at.contains("add(c, k)"), "{after}");
    assert_ne!(after["version"], json!(version));
}

/// FINDING CHECKED: a stored query run on a read-only server goes through
/// the worker (a separate process under the memory bound) and must not
/// touch the store — the worker opens it with `Sqlite::open_read_only`.
/// stdio.rs runs stored queries on a read-only server but never checks the
/// file; this checks the directory and bytes too.
/// Expects: pass.
#[test]
fn read_only_server_runs_stored_queries_without_touching_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let store = sample_store(dir.path());
    let q = "query all_names select p where name(p, d) end";
    {
        let mut c = Client::start(&store, &["--allow-edits"]);
        c.ok("save_query", json!({ "namespace": "sample", "name": "all_names", "text": q }));
    }
    let before_files = dir_listing(dir.path());
    let before_bytes = std::fs::read(&store).unwrap();
    {
        let mut c = Client::start(&store, &[]);
        let t = c.ok("run", json!({ "name": "sample.all_names" }));
        assert!(!t["rows"].as_array().unwrap().is_empty(), "{t}");
        let t = c.ok("query", json!({ "text": q }));
        assert!(!t["rows"].as_array().unwrap().is_empty(), "{t}");
    }
    assert_eq!(dir_listing(dir.path()), before_files);
    assert_eq!(std::fs::read(&store).unwrap(), before_bytes);
}
