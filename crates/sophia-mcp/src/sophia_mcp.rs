//! The agent server (M3b, B-018): Sophia's query API and text operations as
//! Model Context Protocol tools, served on stdio. Agents use the typed query
//! language, never backend SQL (SR-100), and every query runs in a worker
//! process under a hard memory bound (SR-108, [`sophia_query::bounded`]).
//! See `sophia_mcp.md`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::model::{ServerCapabilities, ServerConfig};
use rmcp::{ServerHandler, ServiceExt, schemars, tool, tool_handler, tool_router};
use serde::Deserialize;
use serde_json::{Value as Json, json};
use sophia_hash::Digest;
use sophia_interface::{Direction, EditError, Interface, Key, NodeView, Prop, Subgraph, Written};
use sophia_query::bounded::{Bound, StoreFile, run_bounded};
use sophia_query::{Limits, Prepared, Sort, Strategy, Val};
use sophia_store::sqlite::Sqlite;
use sophia_store::types::Target;

/// Largest `depth` the `context` tool accepts.
pub const MAX_DEPTH: u32 = 3;

/// What a tool may do to the store. Reading is always allowed; each riskier
/// class needs its own flag at startup (SR-157, SR-158), and a tool whose
/// class is not allowed is not listed at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Reads, including running queries: always on.
    Read,
    /// Adds definitions and moves a name to new content (`--allow-edits`).
    Write,
    /// Takes a name away from where it was (`--allow-removals`). The
    /// definitions stay in the store.
    Remove,
}

/// Every tool and its class.
pub const TOOLS: &[(&str, Class)] = &[
    ("query", Class::Read),
    ("run", Class::Read),
    ("show", Class::Read),
    ("list", Class::Read),
    ("context", Class::Read),
    ("edit", Class::Write),
    ("add", Class::Write),
    ("save_query", Class::Write),
    ("rename", Class::Remove),
    ("delete", Class::Remove),
];

/// How the server is started.
#[derive(Debug, Clone)]
pub struct Config {
    /// The SQLite store file. Opened read-only unless a write class is allowed.
    pub store: PathBuf,
    /// The worker binary (`sophia-mcp-worker`) and its memory bound in bytes.
    pub worker: PathBuf,
    pub max_memory: u64,
    /// The ceiling for every query; an agent may ask for fewer rows, not more.
    pub limits: Limits,
    /// Most nodes a `context` or `list` answer holds.
    pub max_nodes: usize,
    /// The reader whose rendering profile `show` prints in (SPEC-004 §4).
    pub reader: String,
    /// The `edit`, `add` and `save_query` tools.
    pub allow_edits: bool,
    /// The `rename` and `delete` tools.
    pub allow_removals: bool,
}

impl Config {
    /// Whether tools of class `c` are served.
    pub fn allows(&self, c: Class) -> bool {
        match c {
            Class::Read => true,
            Class::Write => self.allow_edits,
            Class::Remove => self.allow_removals,
        }
    }

    /// Defaults: 512 MiB per query, 1 000 rows, 10 s, 200 nodes, reader
    /// `agent`, read-only.
    pub fn new(store: impl Into<PathBuf>, worker: impl Into<PathBuf>) -> Self {
        let limits = Limits { max_rows: 1_000, deadline: Some(std::time::Duration::from_secs(10)), ..Limits::default() };
        Config {
            store: store.into(),
            worker: worker.into(),
            max_memory: 512 << 20,
            limits,
            max_nodes: 200,
            reader: "agent".into(),
            allow_edits: false,
            allow_removals: false,
        }
    }
}

struct Inner {
    iface: Interface<Sqlite>,
    store: StoreFile,
    bound: Bound,
    limits: Limits,
    max_nodes: usize,
    config: Config,
}

/// The MCP server. Cheap to clone; clones share one read-only store handle.
#[derive(Clone)]
pub struct SophiaServer {
    inner: Arc<Inner>,
    // The tools of the allowed classes; `#[tool_handler]` serves this field.
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct QueryArgs {
    /// A query in Sophia's query language (SPEC-007), e.g.
    /// `query q select p where name(p, d) && def(d, :Function) end`.
    pub text: String,
    /// At most this many rows (capped by the server's limit).
    pub max_rows: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PathArgs {
    /// A dotted name path, e.g. `sample.add`.
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListArgs {
    /// A dotted namespace path; empty or absent for the root.
    pub namespace: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ContextArgs {
    /// A dotted name path, e.g. `sample.add`.
    pub path: String,
    /// Hops from the name's definition (1 to 3, default 1: the definition
    /// and what it references or is referenced by directly).
    pub depth: Option<u32>,
    /// `out` (what it uses, the default), `in` (what uses it) or `both`.
    pub direction: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RunArgs {
    /// The stored query's dotted name path, or its hex id.
    pub name: String,
    /// Its parameters in order: numbers for Int, strings for Text, hex ids or
    /// name paths for Id.
    pub params: Option<Vec<Json>>,
    /// At most this many rows (capped by the server's limit).
    pub max_rows: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EditArgs {
    /// A dotted name path, e.g. `sample.add`.
    pub path: String,
    /// The version `show` returned with the text being edited.
    pub version: u64,
    /// The new text of the whole definition (its whole recursive group).
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddArgs {
    /// A dotted namespace path; empty for the root.
    pub namespace: String,
    /// The new name: one segment, no dots.
    pub name: String,
    /// The definition's text.
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RenameArgs {
    /// The dotted name path to move.
    pub path: String,
    /// The new dotted path.
    pub to: String,
}

impl SophiaServer {
    /// Opens the store: read-only unless `allow_edits` or `allow_removals`.
    /// Fails if the file does not exist or is not a Sophia SQLite store; the
    /// server never creates a store.
    pub fn open(config: Config) -> Result<Self, String> {
        if !config.store.is_file() {
            return Err(format!("no store file at {}", config.store.display()));
        }
        let writes = config.allow_edits || config.allow_removals;
        let iface = if writes {
            let backend = Sqlite::open(&config.store).map_err(|e| e.to_string())?;
            Interface::open(backend, config.reader.clone()).map_err(|e| e.to_string())?
        } else {
            let backend = Sqlite::open_read_only(&config.store).map_err(|e| e.to_string())?;
            Interface::open_read_only(backend, config.reader.clone())
        };
        let mut router = Self::tool_router();
        for (name, class) in TOOLS {
            if !config.allows(*class) {
                router.remove_route(name);
            }
        }
        let inner = Inner {
            iface,
            store: StoreFile::Sqlite(config.store.clone()),
            bound: Bound::new(config.worker.clone(), config.max_memory),
            limits: config.limits,
            max_nodes: config.max_nodes,
            config,
        };
        Ok(SophiaServer { inner: Arc::new(inner), tool_router: router })
    }

    /// Refuses a call of a class the server was not started for. The router
    /// already leaves such tools out; this is the second lock.
    fn permit(&self, c: Class) -> Result<(), String> {
        if self.inner.config.allows(c) {
            Ok(())
        } else {
            let flag = if c == Class::Write { "--allow-edits" } else { "--allow-removals" };
            Err(format!("this server was started without {flag}"))
        }
    }

    /// Serves MCP on stdin and stdout until the client closes the stream.
    pub async fn serve_stdio(self) -> Result<(), String> {
        let running = self.serve(rmcp::transport::stdio()).await.map_err(|e| e.to_string())?;
        running.waiting().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The `query` tool's work, synchronous (it waits for the worker).
    pub fn run_query(&self, text: &str, max_rows: Option<usize>) -> Result<Json, String> {
        let p = sophia_query::prepare_text(text).map_err(|e| format!("query refused: {e}"))?;
        if !p.query.params.is_empty() {
            return Err("a query with parameters cannot run as text: store it with save_query and call it with run".into());
        }
        self.run_prepared(&p, &[], max_rows)
    }

    /// The `run` tool's work: a stored query by name or id, with its
    /// parameters in order. An `Id` parameter is a hex id or a name path.
    pub fn run_stored(&self, name: &str, params: &[Json], max_rows: Option<usize>) -> Result<Json, String> {
        let store = self.inner.iface.store();
        let p = sophia_query::load(store, name).map_err(|e| format!("cannot load {name}: {e}"))?;
        let sorts = &p.query.params;
        if params.len() != sorts.len() {
            let want: Vec<&str> = sorts.iter().map(|s| s.as_str()).collect();
            return Err(format!("{name} takes {} parameter(s) ({}), given {}", sorts.len(), want.join(", "), params.len()));
        }
        let mut vals = Vec::with_capacity(params.len());
        for (k, (v, s)) in params.iter().zip(sorts).enumerate() {
            let bad = || format!("parameter {} must be {}: got {v}", k + 1, s.as_str());
            vals.push(match (s, v) {
                (Sort::Int, Json::Number(n)) => Val::Int(n.as_i64().ok_or_else(bad)?),
                (Sort::Text, Json::String(t)) => Val::Text(t.clone()),
                (Sort::Id, Json::String(t)) => match t.parse::<Digest>() {
                    Ok(d) => Val::Id(d),
                    Err(_) => match store.resolve(t).map_err(|e| e.to_string())?.map(|n| n.target) {
                        // A definition or a nominal entity: both are `Id`s (SPEC-007 §2).
                        Some(Target::Content(d) | Target::Nominal(d)) => Val::Id(d),
                        None => return Err(format!("parameter {}: no name {t}", k + 1)),
                    },
                },
                _ => return Err(bad()),
            });
        }
        self.run_prepared(&p, &vals, max_rows)
    }

    fn run_prepared(&self, p: &Prepared, params: &[Val], max_rows: Option<usize>) -> Result<Json, String> {
        let i = &self.inner;
        let mut limits = i.limits;
        if let Some(m) = max_rows {
            limits.max_rows = m.min(limits.max_rows);
        }
        let rows = run_bounded(&i.bound, &i.store, p, params, limits, Strategy::Auto).map_err(|e| format!("query failed: {e}"))?;
        // Id cells print as hex; `ids` says what each one is.
        let mut ids = BTreeMap::new();
        let out: Vec<Json> = rows
            .rows
            .iter()
            .map(|r| {
                Json::Array(
                    r.iter()
                        .map(|v| match v {
                            Val::Id(d) => {
                                let hex = d.to_hex();
                                if !ids.contains_key(&hex) {
                                    let what = match i.iface.node(&Key::Content(*d)) {
                                        Ok(Some(n)) => json!({ "kind": n.kind, "label": n.label }),
                                        _ => Json::Null,
                                    };
                                    ids.insert(hex.clone(), what);
                                }
                                Json::String(hex)
                            }
                            Val::Text(s) => Json::String(s.clone()),
                            Val::Int(n) => json!(n),
                        })
                        .collect(),
                )
            })
            .collect();
        Ok(json!({ "columns": rows.columns, "rows": out, "truncated": rows.truncated, "ids": ids }))
    }

    /// The `show` tool's work.
    pub fn show_text(&self, path: &str) -> Result<Json, String> {
        let (text, version) = self.inner.iface.fetch_text(&Key::name(path)).map_err(|e| e.to_string())?;
        Ok(json!({ "path": path, "version": version, "text": text }))
    }

    /// The `list` tool's work.
    pub fn list_namespace(&self, namespace: &str) -> Result<Json, String> {
        let key = if namespace.is_empty() { self.inner.iface.root() } else { Key::ns(namespace) };
        let mut g = self.inner.iface.children(&key).map_err(|e| e.to_string())?;
        self.cap(&mut g);
        Ok(json!({
            "namespace": namespace,
            "children": g.nodes.iter().filter(|n| n.kind != "sophia:Settings").map(|n| json!({ "kind": n.kind, "label": n.label })).collect::<Vec<_>>(),
            "truncated": g.truncated,
        }))
    }

    /// The `context` tool's work.
    pub fn context_of(&self, path: &str, depth: Option<u32>, direction: Option<&str>) -> Result<Json, String> {
        let depth = depth.unwrap_or(1);
        if !(1..=MAX_DEPTH).contains(&depth) {
            return Err(format!("depth must be 1 to {MAX_DEPTH}"));
        }
        let dir = match direction.unwrap_or("out") {
            "out" => Direction::Out,
            "in" => Direction::In,
            "both" => Direction::Both,
            d => return Err(format!("direction must be out, in or both, not {d:?}")),
        };
        let i = &self.inner.iface;
        let key = Key::name(path);
        let name = i.node(&key).map_err(|e| e.to_string())?.ok_or_else(|| format!("no name {path}"))?;
        // Walk from what the name points to: from the name itself, `in`
        // would only reach the namespaces above it (Codex's review, B-062).
        let first = i.neighbours(&key, 1, Direction::Out, self.inner.max_nodes).map_err(|e| e.to_string())?;
        let names = first.edges.into_iter().find(|e| e.from == key && e.kind == "Names").ok_or_else(|| format!("{path} names nothing"))?;
        let mut g = i.neighbours(&names.to, depth, dir, self.inner.max_nodes).map_err(|e| e.to_string())?;
        if !g.nodes.iter().any(|n| n.key == key) {
            g.nodes.insert(0, name);
        }
        if !g.edges.contains(&names) {
            g.edges.insert(0, names);
        }
        Ok(subgraph_json(&g))
    }

    /// The `edit` tool's work: replace what `path` names by `text`, if the
    /// name is still at `version` (from `show`).
    pub fn edit_text(&self, path: &str, version: u64, text: &str) -> Result<Json, String> {
        self.permit(Class::Write)?;
        let w = self.inner.iface.write_text(&Key::name(path), version, text).map_err(|e| edit_error(&e, text))?;
        Ok(written_json(path, &w))
    }

    /// The `add` and `save_query` tools' work: new definitions under
    /// `namespace` (empty: the root), named `name`.
    pub fn add_text(&self, namespace: &str, name: &str, text: &str, query_only: bool) -> Result<Json, String> {
        self.permit(Class::Write)?;
        if name.is_empty() || name.contains(['.', '/']) {
            return Err(format!("name must be one segment, without . or /: {name:?}"));
        }
        if query_only {
            let first = text.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#')).unwrap_or("");
            if !(first.starts_with("query ") || first.starts_with("select ")) {
                return Err("a query starts with `query` or `select`".into());
            }
        }
        let parent = if namespace.is_empty() { self.inner.iface.root() } else { Key::ns(namespace) };
        let (key, mut w) = self.inner.iface.create_text(&parent, name, text).map_err(|e| edit_error(&e, text))?;
        // The name the text defined (it may not be `name`), and its version
        // token: `create_text` gives 0 for code, which `edit` would refuse
        // (Codex's review of B-063).
        let path = match &key {
            Key::Derived(s) => s.strip_prefix("name:").unwrap_or(s).to_string(),
            Key::Content(d) => d.to_hex(),
        };
        w.version = self.inner.iface.text_token(&path).map_err(|e| e.to_string())?;
        Ok(written_json(&path, &w))
    }

    /// The `rename` tool's work.
    pub fn rename_name(&self, path: &str, to: &str) -> Result<Json, String> {
        self.permit(Class::Remove)?;
        let key = self.inner.iface.rename(&Key::name(path), &sophia_interface::path_to_moonkale(to)).map_err(|e| edit_error(&e, ""))?;
        let new = match key {
            Key::Derived(s) => s.strip_prefix("name:").unwrap_or(&s).to_string(),
            Key::Content(d) => d.to_hex(),
        };
        Ok(json!({ "from": path, "to": new }))
    }

    /// The `delete` tool's work: the name goes, the definition stays.
    pub fn delete_name(&self, path: &str) -> Result<Json, String> {
        self.permit(Class::Remove)?;
        self.inner.iface.delete(&Key::name(path)).map_err(|e| edit_error(&e, ""))?;
        Ok(json!({ "deleted": path }))
    }

    fn cap(&self, g: &mut Subgraph) {
        if g.nodes.len() > self.inner.max_nodes {
            g.nodes.truncate(self.inner.max_nodes);
            g.truncated = true;
        }
    }
}

/// Nodes with a short reference each (`n0`, `n1`, …) and edges between those
/// references, so an agent reads labels, not 64-digit ids.
fn subgraph_json(g: &Subgraph) -> Json {
    let refs: BTreeMap<&Key, String> = g.nodes.iter().enumerate().map(|(k, n)| (&n.key, format!("n{k}"))).collect();
    let nodes: Vec<Json> = g.nodes.iter().map(|n| node_json(n, &refs[&n.key])).collect();
    let edges: Vec<Json> =
        g.edges.iter().filter_map(|e| Some(json!({ "from": refs.get(&e.from)?, "kind": e.kind, "to": refs.get(&e.to)? }))).collect();
    json!({ "nodes": nodes, "edges": edges, "truncated": g.truncated })
}

fn node_json(n: &NodeView, r: &str) -> Json {
    let mut o = json!({ "ref": r, "kind": n.kind, "label": n.label });
    if let Key::Content(d) = &n.key {
        o["id"] = Json::String(d.to_hex());
    }
    for (k, v) in &n.props {
        if matches!(k.as_str(), "lang" | "sophialang_kind" | "def_kind") {
            o[k] = match v {
                Prop::Text(s) => json!(s),
                Prop::Int(i) => json!(i),
                Prop::Bool(b) => json!(b),
            };
        }
    }
    o
}

fn written_json(path: &str, w: &Written) -> Json {
    json!({
        "path": path,
        "version": w.version,
        "targets": w.targets.iter().map(|(n, d)| json!({ "name": n, "id": d.to_hex() })).collect::<Vec<_>>(),
        "warnings": w.warnings.iter().map(|d| json!({ "start": d.span.start, "end": d.span.end, "message": d.message })).collect::<Vec<_>>(),
        "stale_dependants": w.stale_dependants.iter().map(|d| d.to_hex()).collect::<Vec<_>>(),
    })
}

/// An edit's failure as JSON text the agent can act on: diagnostics with
/// byte spans into its text and the line they start on, or the version
/// conflict.
fn edit_error(e: &EditError, text: &str) -> String {
    let j = match e {
        EditError::Diagnostics(ds) => json!({
            "error": "diagnostics",
            "diagnostics": ds.iter().map(|d| json!({
                "severity": format!("{:?}", d.severity).to_lowercase(),
                "start": d.span.start,
                "end": d.span.end,
                "line": text.get(..d.span.start.min(text.len())).map(|t| t.matches('\n').count() + 1),
                "message": d.message,
            })).collect::<Vec<_>>(),
        }),
        EditError::Conflict { expected, found } => json!({
            "error": "conflict",
            "message": "changed since it was read: show it again and redo the edit",
            "expected": expected,
            "found": found,
        }),
        EditError::Refused(m) => json!({ "error": "refused", "message": m }),
        EditError::Store(e) => json!({ "error": "store", "message": e.to_string() }),
    };
    serde_json::to_string_pretty(&j).unwrap_or_default()
}

fn render(r: Result<Json, String>) -> Result<String, String> {
    r.map(|j| serde_json::to_string_pretty(&j).unwrap_or_default())
}

/// Runs blocking store work off the async runtime's threads.
async fn blocking(server: &SophiaServer, f: impl FnOnce(&SophiaServer) -> Result<Json, String> + Send + 'static) -> Result<String, String> {
    let s = server.clone();
    render(tokio::task::spawn_blocking(move || f(&s)).await.map_err(|e| format!("internal error: {e}"))?)
}

#[tool_router]
impl SophiaServer {
    #[tool(
        description = "Run a read-only query in Sophia's query language (Datalog with a Julia-flavoured syntax, SPEC-007) under a hard memory bound. Base relations include name(path, id), def(id, kind), refs(a, b). Returns columns, rows, and for every id cell its kind and label.",
        annotations(read_only_hint = true)
    )]
    async fn query(&self, Parameters(a): Parameters<QueryArgs>) -> Result<String, String> {
        blocking(self, move |s| s.run_query(&a.text, a.max_rows)).await
    }

    #[tool(
        description = "Run a stored query by name or id, with its parameters in order (an Id parameter is a hex id or a name path), under the same memory bound as query.",
        annotations(read_only_hint = true)
    )]
    async fn run(&self, Parameters(a): Parameters<RunArgs>) -> Result<String, String> {
        blocking(self, move |s| s.run_stored(&a.name, &a.params.unwrap_or_default(), a.max_rows)).await
    }

    #[tool(
        description = "Replace the definition a name points to with new text (sophialang, or query text for a stored query). Give the version from show: if the name changed since, the edit is refused with a conflict. Parse errors come back as diagnostics with byte spans; nothing is stored then. Needs --allow-edits.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn edit(&self, Parameters(a): Parameters<EditArgs>) -> Result<String, String> {
        blocking(self, move |s| s.edit_text(&a.path, a.version, &a.text)).await
    }

    #[tool(
        description = "Add new definitions from text under a namespace, named name (one segment). Needs --allow-edits.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn add(&self, Parameters(a): Parameters<AddArgs>) -> Result<String, String> {
        blocking(self, move |s| s.add_text(&a.namespace, &a.name, &a.text, false)).await
    }

    #[tool(
        description = "Store a query (text starting with query or select) as a definition under namespace.name, so run can call it with parameters. Needs --allow-edits.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn save_query(&self, Parameters(a): Parameters<AddArgs>) -> Result<String, String> {
        blocking(self, move |s| s.add_text(&a.namespace, &a.name, &a.text, true)).await
    }

    #[tool(
        description = "Move a name to a new dotted path; the definition is unchanged. Needs --allow-removals.",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn rename(&self, Parameters(a): Parameters<RenameArgs>) -> Result<String, String> {
        blocking(self, move |s| s.rename_name(&a.path, &a.to)).await
    }

    #[tool(
        description = "Remove a name. The definition stays in the store (content is never deleted). Needs --allow-removals.",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn delete(&self, Parameters(a): Parameters<PathArgs>) -> Result<String, String> {
        blocking(self, move |s| s.delete_name(&a.path)).await
    }

    #[tool(
        description = "Print the definition a name points to, in the server's rendering profile, with its version token (the token a later edit must quote).",
        annotations(read_only_hint = true)
    )]
    async fn show(&self, Parameters(a): Parameters<PathArgs>) -> Result<String, String> {
        blocking(self, move |s| s.show_text(&a.path)).await
    }

    #[tool(
        description = "List a namespace's sub-namespaces and names (the root when no namespace is given).",
        annotations(read_only_hint = true)
    )]
    async fn list(&self, Parameters(a): Parameters<ListArgs>) -> Result<String, String> {
        blocking(self, move |s| s.list_namespace(a.namespace.as_deref().unwrap_or(""))).await
    }

    #[tool(
        description = "The neighbourhood of a name: the name, its definition, and what the definition references (direction out, the default), what references it (in), or both, up to depth hops from the definition (1 to 3, default 1), as nodes and edges.",
        annotations(read_only_hint = true)
    )]
    async fn context(&self, Parameters(a): Parameters<ContextArgs>) -> Result<String, String> {
        blocking(self, move |s| s.context_of(&a.path, a.depth, a.direction.as_deref())).await
    }
}

// The router field, not the default `Self::tool_router()`: the field has
// the tools of disallowed classes removed.
#[tool_handler(router = self.tool_router)]
impl ServerHandler for SophiaServer {
    fn get_info(&self) -> ServerConfig {
        let c = &self.inner.config;
        let mut text = String::from(
            "Sophia, a content-addressed code store. Names are dotted paths (sample.add). Read with list, show and context; \
             ask questions with query (Sophia's query language, not SQL) and run stored queries with run.",
        );
        text.push_str(match (c.allow_edits, c.allow_removals) {
            (false, false) => " This server is read-only.",
            (true, false) => " Edit with show then edit (quote the version), add and save_query. Names cannot be renamed or deleted here.",
            (false, true) => " Names can be renamed and deleted here, but not edited.",
            (true, true) => " Edit with show then edit (quote the version), add and save_query; rename and delete move or remove names.",
        });
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(text)
    }
}
