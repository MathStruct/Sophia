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
use sophia_interface::{Direction, Interface, Key, NodeView, Prop, Subgraph};
use sophia_query::bounded::{Bound, StoreFile, run_bounded};
use sophia_query::{Limits, Strategy, Val};
use sophia_store::sqlite::Sqlite;

/// Largest `depth` the `context` tool accepts.
pub const MAX_DEPTH: u32 = 3;

/// How the server is started.
#[derive(Debug, Clone)]
pub struct Config {
    /// The SQLite store file. The server opens it read-only.
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
}

impl Config {
    /// Defaults: 512 MiB per query, 1 000 rows, 10 s, 200 nodes, reader `agent`.
    pub fn new(store: impl Into<PathBuf>, worker: impl Into<PathBuf>) -> Self {
        let limits = Limits { max_rows: 1_000, deadline: Some(std::time::Duration::from_secs(10)), ..Limits::default() };
        Config { store: store.into(), worker: worker.into(), max_memory: 512 << 20, limits, max_nodes: 200, reader: "agent".into() }
    }
}

struct Inner {
    iface: Interface<Sqlite>,
    store: StoreFile,
    bound: Bound,
    limits: Limits,
    max_nodes: usize,
}

/// The MCP server. Cheap to clone; clones share one read-only store handle.
#[derive(Clone)]
pub struct SophiaServer {
    inner: Arc<Inner>,
    // Read by the code `#[tool_handler]` generates.
    #[allow(dead_code)]
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

impl SophiaServer {
    /// Opens the store read-only. Fails if the file does not exist or is not
    /// a Sophia SQLite store.
    pub fn open(config: Config) -> Result<Self, String> {
        if !config.store.is_file() {
            return Err(format!("no store file at {}", config.store.display()));
        }
        let backend = Sqlite::open_read_only(&config.store).map_err(|e| e.to_string())?;
        let iface = Interface::open_read_only(backend, config.reader);
        let inner = Inner {
            iface,
            store: StoreFile::Sqlite(config.store),
            bound: Bound::new(config.worker, config.max_memory),
            limits: config.limits,
            max_nodes: config.max_nodes,
        };
        Ok(SophiaServer { inner: Arc::new(inner), tool_router: Self::tool_router() })
    }

    /// Serves MCP on stdin and stdout until the client closes the stream.
    pub async fn serve_stdio(self) -> Result<(), String> {
        let running = self.serve(rmcp::transport::stdio()).await.map_err(|e| e.to_string())?;
        running.waiting().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The `query` tool's work, synchronous (it waits for the worker).
    pub fn run_query(&self, text: &str, max_rows: Option<usize>) -> Result<Json, String> {
        let i = &self.inner;
        let p = sophia_query::prepare_text(text).map_err(|e| format!("query refused: {e}"))?;
        let mut limits = i.limits;
        if let Some(m) = max_rows {
            limits.max_rows = m.min(limits.max_rows);
        }
        let rows = run_bounded(&i.bound, &i.store, &p, &[], limits, Strategy::Auto).map_err(|e| format!("query failed: {e}"))?;
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
        description = "Run a read-only query in Sophia's query language (Datalog with a Julia-flavoured syntax, SPEC-007) under a hard memory bound. Base relations include name(path, id), def(id, kind), refs(a, b). Returns columns, rows, and for every id cell its kind and label."
    )]
    async fn query(&self, Parameters(a): Parameters<QueryArgs>) -> Result<String, String> {
        blocking(self, move |s| s.run_query(&a.text, a.max_rows)).await
    }

    #[tool(
        description = "Print the definition a name points to, in the server's rendering profile, with its version token (the token a later edit must quote)."
    )]
    async fn show(&self, Parameters(a): Parameters<PathArgs>) -> Result<String, String> {
        blocking(self, move |s| s.show_text(&a.path)).await
    }

    #[tool(description = "List a namespace's sub-namespaces and names (the root when no namespace is given).")]
    async fn list(&self, Parameters(a): Parameters<ListArgs>) -> Result<String, String> {
        blocking(self, move |s| s.list_namespace(a.namespace.as_deref().unwrap_or(""))).await
    }

    #[tool(
        description = "The neighbourhood of a name: the name, its definition, and what the definition references (direction out, the default), what references it (in), or both, up to depth hops from the definition (1 to 3, default 1), as nodes and edges."
    )]
    async fn context(&self, Parameters(a): Parameters<ContextArgs>) -> Result<String, String> {
        blocking(self, move |s| s.context_of(&a.path, a.depth, a.direction.as_deref())).await
    }
}

#[tool_handler]
impl ServerHandler for SophiaServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Sophia, a content-addressed code store. Names are dotted paths (sample.add). Read with list, show and context; \
             ask questions with query (Sophia's query language, not SQL). This server is read-only.",
        )
    }
}
