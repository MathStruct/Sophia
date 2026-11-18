#implementation

> The agent server (M3b, B-018; first slice B-062): Sophia's query language and its read operations as Model Context Protocol tools on stdio, built on the official Rust SDK `rmcp` (SR-156). Queries run in a worker process under a hard memory bound ([[bounded]]). Read-only so far. Source: `sophia_mcp.rs`.
>
> Sources: the Model Context Protocol; `rmcp` 3.11 (github.com/modelcontextprotocol/rust-sdk); [[LLMs and Mixed-Language Code]] §2; [[SPEC-007 Sophia Query Language]] §8; SR-62, SR-100, SR-108. code: `sophia_mcp.rs`, `bin/sophia-mcp.rs`, `bin/sophia-mcp-worker.rs`, `tests/stdio.rs`

## What it does

`SophiaServer::open(Config)` opens a **SQLite** store file **read-only** (`Sqlite::open_read_only`, `Interface::open_read_only`) and serves four tools. Every answer is JSON text; every failure is a tool error (`isError: true`) with a message, so the agent sees it and the session goes on.

| tool | arguments | what it returns | through |
|---|---|---|---|
| `query` | `text`, `max_rows?` | `columns`, `rows`, `truncated`, and `ids`: for every id cell its node `kind` and `label` | `prepare_text`, then `run_bounded` in the worker |
| `show` | `path` | the printed text in the reader's profile and its `version` token | `Interface::fetch_text` |
| `list` | `namespace?` | the sub-namespaces and names (`kind`, `label`), the settings node left out | `Interface::children` |
| `context` | `path`, `depth?` (1 to 3 hops from the name's definition, default 1), `direction?` (`out` default, `in`, `both`) | the name, its definition and its neighbours: nodes with short refs (`n0`, …), `kind`, `label`, `id`, `lang`, `def_kind`; edges between refs | `Interface::neighbours` from the definition the name points to |

- **No SQL** (SR-100): agents get Sophia's query language only. The raw `sql` dialect is not exposed.
- **Every query is bounded** (SR-108): it runs in `sophia-mcp-worker` under `Config::max_memory` (default 512 MiB) and the server's `Limits` (default: 1 000 rows, 10 s, the rest `Limits::default`). An agent's `max_rows` can lower the row cap, not raise it. A query with parameters cannot run as text (as in `Interface::query`).
- **The other three tools** read in the server process, without the worker: they are bounded by `Config::max_nodes` (200) and by what they read, not by a query an agent writes.
- **Blocking work** (the store, waiting for the worker) runs on `spawn_blocking`, off the async runtime's threads.

## Choices (2026-11-18)

- **`rmcp`**: Daniel's choice (SR-156), over writing the JSON-RPC ourselves or a community SDK. Default features off; `server`, `macros`, `transport-io`. It needs Rust 1.88, so this crate's `rust-version` is 1.88 (the workspace's is 1.85).
- **stdio, read-only first, SQLite only**: Claude's proposal, not yet confirmed by Daniel. stdio means one agent per server process and no network surface. SQLite (WAL) lets the worker read while another process writes, which redb and DuckDB do not ([[bounded]]).
- **The worker is this crate's own binary**, `sophia-mcp-worker` (its `main` is `bounded::worker_main`), because cargo does not build a dependency's binaries. The server looks for it beside its own executable unless `--worker` names it.

## What it does not do yet

- **Edits** (`edit`, `add`) and **stored queries** (save, run by name with parameters): B-063.
- **Other backends** and **facts** (typings, test results, proofs; tags and attributes): B-064.
- **Latency** by dialect and backend (B-11): B-065.
- `context` caps the nodes at `max_nodes` inside `neighbours`; it does not say which hops were cut.

## Tests (`tests/stdio.rs`)

The real binary on stdio, spoken to in raw JSON-RPC lines (not through `rmcp`'s client, so the test also checks the wire format), on a SQLite store with the sample program (SPEC-004 §9):
- `tools/list` names exactly the four tools;
- `query` finds `sample.add_test` among the transitive users of `sample.add` and describes every id cell; `max_rows` caps the rows;
- `show`, `list` (a namespace and the root) and `context` (the edges join nodes that are in the answer; `References` among them);
- `context` with `in` and `both` finds `sample.add_test` as a user of `sample.add` and keeps the name (Codex's finding; before the fix `in` walked from the name to its namespace);
- six bad calls (missing name, unknown relation, text that is not a query, depth 9, direction `up`, `context` of a missing name) are tool errors, and the server answers after them;
- with `--max-memory 0` every query fails with the bound's error while `show` still works: queries go through the worker;
- a missing store file exits 1, a missing `--store` exits 2.

`cargo test -p sophia-mcp`: 6 passed (2026-11-18).

## Review

Codex reviewed the change (2026-11-18, `codex exec review --uncommitted`) and found one defect: `context` with `direction: "in"` started from the name node, whose only incoming edge is `Contains` from its namespace, so it never reached the users. Reproduced by `context_in_finds_the_users` (it failed: the answer held the name and `sample`), fixed by walking from the definition the name points to.
