#implementation

> The agent server (M3b, B-018; B-062 to B-064): Sophia's query language, its read operations, the facts and annotations recorded about a definition, and its text edits as Model Context Protocol tools on stdio, built on the official Rust SDK `rmcp` (SR-156), on any of the four backends ([[backends]]). Queries run in a worker process under a hard memory bound ([[bounded]]). Tools are grouped by what they may do to the store; the riskier groups are served only with a startup flag (SR-157, SR-158); on redb and DuckDB only reading is served (SR-159). Source: `sophia_mcp.rs`.
>
> Sources: the Model Context Protocol; `rmcp` 3.11 (github.com/modelcontextprotocol/rust-sdk); [[LLMs and Mixed-Language Code]] §2; [[SPEC-004 Moonkale Interface]] §3; [[SPEC-007 Sophia Query Language]] §8; SR-62, SR-100, SR-108. code: `sophia_mcp.rs`, `backends.rs`, `bin/sophia-mcp.rs`, `bin/sophia-mcp-worker.rs`, `tests/stdio.rs`, `tests/backends.rs`, `tests/review_mistral.rs`

## Permission classes

Every tool has a `Class` (the table `TOOLS`). Reading is always allowed; each riskier class needs its own flag (Daniel, 2026-11-18: "allow flags for the more critical ones, as a general measure"):

| class | tools | flag | what it can do to the store |
|---|---|---|---|
| `Read` | `query`, `run`, `show`, `list`, `context`, `facts` | none | nothing |
| `Write` | `edit`, `add`, `save_query` | `--allow-edits` | add definitions; move a name to new content (the old content stays) |
| `Remove` | `rename`, `delete` | `--allow-removals` | take a name away from where it was (the definition stays) |

- A tool whose class is not allowed is **not listed** (`tools/list`) and **cannot be called**: it is removed from the router at startup, and each write method checks the class again (`permit`), so a missing route is not the only lock.
- The flags are independent: `--allow-removals` does not allow edits, nor the other way round.
- **redb and DuckDB serve only `Read`** (SR-159): either write flag is refused at startup, because the worker cannot open their file while the server holds it for writing ([[backends]]).
- Without either flag the store is opened **read-only** (each backend's `open_read_only`, `Interface::open_read_only`); a session changes no byte of the store's data file, on every backend (tested). SQLite creates its side files `-shm` and an empty `-wal` beside it, as for every reader of a WAL-mode file; nothing is written to them ([[Review 2026-11-18 sophia-mcp (external)]] 5). With one, it is opened read-write (`Interface::open`, which writes the built-in rendering profiles if they are missing). The server never creates a store: it first opens every store read-only and reads it, so a file that is not a Sophia store is refused before a read-write open could initialise it.
- Content is never deleted by any tool: the store is append-only for definitions.
- The MCP annotations say the same to the client: `read_only_hint` on the read tools, `destructive_hint` on `rename` and `delete`.
- **New tools must be added to `TOOLS`** with their class; a tool missing from the table is served unconditionally. (A test lists the tools per flag combination and would show it.)

## The tools

Every answer is JSON text; every failure is a tool error (`isError: true`) with a message, so the agent sees it and the session goes on.

| tool | arguments | what it returns | through |
|---|---|---|---|
| `query` | `text`, `max_rows?` | `columns`, `rows`, `truncated`, and `ids`: for every id cell its node `kind` and `label` | `prepare_text`, then `run_bounded` in the worker |
| `run` | `name` (path or hex id), `params?` (in order: number for `Int`, string for `Text`, hex id or name path for `Id`; a name may point at a definition or a nominal entity), `max_rows?` | as `query` | `sophia_query::load`, then `run_bounded` |
| `show` | `path` | the printed text in the reader's profile and its `version` token | `Interface::fetch_text` |
| `list` | `namespace?` | the sub-namespaces and names (`kind`, `label`), the settings node left out | `Interface::children` |
| `facts` | `path` (name path or hex id), `kind?` | `subject`; `facts`: `id`, `kind`, `value`, `by`, `by_version`, `derived_at`, `stale`, `inputs` (`{content}` or `{relation, version}`); `annotations`: `id`, `kind`, `payload`, `status`, `checker`, `checker_version`, `with` (the annotation's other subjects); `truncated` | `facts_of`, `Store::is_stale`, `annotations_of` on what the name points to (a definition or a nominal entity) |
| `context` | `path`, `depth?` (1 to 3 hops from the name's definition, default 1), `direction?` (`out` default, `in`, `both`) | the name, its definition and its neighbours: nodes with short refs (`n0`, …), `kind`, `label`, `id`, `lang`, `def_kind`; edges between refs | `Interface::neighbours` from the definition the name points to |
| `edit` | `path`, `version` (from `show`), `text` | the new `version`, `targets` (name → id), `warnings`, `stale_dependants` | `Interface::write_text` |
| `add` | `namespace` (empty: root), `name` (one segment), `text` | as `edit`; `path` is the name the text defined (`name` if it defines it, else its first name) and `version` that name's token, ready for `edit` | `Interface::create_text`, then `text_token` |
| `save_query` | as `add`; the text must start with `query` or `select` | as `edit` | `Interface::create_text` (it stores query text as a query, SPEC-007 §8) |
| `rename` | `path`, `to` (dotted) | `from`, `to` | `Interface::rename` |
| `delete` | `path` | `deleted` | `Interface::delete` |

- **No SQL** (SR-100): agents get Sophia's query language only. The raw `sql` dialect is not exposed.
- **Every query is bounded** (SR-108): `query` and `run` run in `sophia-mcp-worker` under `Config::max_memory` (default 512 MiB) and the server's `Limits` (default: 1 000 rows, 10 s, the rest `Limits::default`). An agent's `max_rows` can lower the row cap, not raise it. Query text with parameters is refused by `query` with a pointer to `save_query` and `run`.
- **Edit failures** are JSON the agent can act on: `{"error": "diagnostics", "diagnostics": [{severity, start, end, line, message}]}` (byte spans into the text it sent; nothing stored), `{"error": "conflict", expected, found}` (the name changed since `show`), `refused` or `store`.
- **The worker sees the server's writes**: SQLite in WAL mode, the server's transaction committed before the tool answers (tested: a name added through `add` is found by `query`).
- **`facts`** reads the two relations of SPEC-007 §2: derived facts (`fact`: typings, test results, dispatch resolutions, source spans, …) and annotations (`annotation`: proofs, tags, attributes, docs, …). There is no separate tag or attribute relation: a tag is an annotation of kind `tag`, an attribute one of its own kind, so `kind` selects them. A value or payload prints as text when it is UTF-8, else as `{"hex": …}`. `stale` is true when a relation the fact depended on has a newer version (SPEC-001 §8). Each list, and each list inside an entry (`inputs`, `with`), holds at most `max_nodes` entries; `truncated` says one was cut.
- **The read tools other than `query` and `run`** read in the server process, without the worker: they are bounded by `Config::max_nodes` (200) and by what they read, not by a query an agent writes.
- **Blocking work** (the store, waiting for the worker) runs on `spawn_blocking`, off the async runtime's threads.

## Choices (2026-11-18)

- **`rmcp`**: Daniel's choice (SR-156). Default features off; `server`, `macros`, `transport-io`. It needs Rust 1.88, so this crate's `rust-version` is 1.88 (the workspace's is 1.85).
- **The tools and the flags**: Daniel's (SR-157, SR-158).
- **stdio**: Claude's proposal, not yet confirmed by Daniel. One agent per server process and no network surface.
- **Every backend, redb and DuckDB read-only** (B-064): Daniel's choice (SR-159), over closing the store around each query or handing the worker a copy ([[backends]]). `--backend` picks it; SQLite is the default.
- **The worker is this crate's own binary**, `sophia-mcp-worker` (its `main` is `bounded::worker_main`), because cargo does not build a dependency's binaries. The server looks for it beside its own executable unless `--worker` names it.
- **`#[tool_handler(router = self.tool_router)]`**: the macro's default builds a fresh router per request (`Self::tool_router()`), which would serve every tool and ignore the classes. Found by `tools_are_listed_by_class`.

## What it does not do yet

- **Latency** by dialect and backend (B-11): B-065.
- Edits through a generated file (`propose_text`, `accept_text`): not exposed; B-060 first.
- `context` caps the nodes at `max_nodes` inside `neighbours`; it does not say which hops were cut.

## Tests (`tests/backends.rs`)

The same sample on every backend the build has (all four with `--all-features`), with a stored query, a current `test-result` fact and a stale `typing` fact on `sample.add_test`, and a `tag` and a `proof` annotation:
- **read** (`every_backend_serves_the_read_tools`): a read-only server lists the six read tools; `query` and `run` (the worker, with the server holding the store) find `sample.add_test`; `show`, `list`, `context`; `facts` gives both facts with value, producer, inputs and staleness, a non-UTF-8 value as hex, the proof's other subject in `with`, the same answer by hex id, one kind with `kind`, a tag on `sample.add`, nothing on a definition without facts, an error on a missing name; the store's data file is byte-identical after the session;
- **writes** (`lmdb_serves_writes`): on LMDB with both flags, `add` is seen by the worker's `query`, `edit` and `delete` work;
- **refused** (`locking_backends_refuse_the_write_flags`): redb and DuckDB with either flag exit 1 and leave the file unchanged;
- **start** (`missing_stores_and_unknown_backends`): a missing store exits 1 on each backend and creates nothing; an LMDB directory given as SQLite exits 1; an unknown backend exits 2;
- **review findings** (`review_findings_b064_*`): the lists inside a fact and an annotation are capped at `max_nodes` and set `truncated`; an empty file is "not a Sophia … store" on every backend, with or without `--allow-edits`, and stays empty.

## Tests (`tests/stdio.rs`)

The real binary on stdio, spoken to in raw JSON-RPC lines (not through `rmcp`'s client, so the test also checks the wire format), on a SQLite store with the sample program (SPEC-004 §9):
- **classes:** `tools/list` per flag combination (none, edits, removals, both) lists exactly the allowed classes; without flags every write tool called by name is refused and the store file is byte-identical after the session; removals alone refuse `edit`, edits alone refuse `delete`;
- **read:** `query` finds `sample.add_test` among the transitive users of `sample.add` and describes every id cell; `max_rows` caps the rows; `show`, `list` (a namespace and the root), `context` out, in and both;
- **edit:** a parse error returns diagnostics with a line and a span past the original text and stores nothing; a stale version is a `conflict` naming the current one; the right version stores the change, `show` prints it, and the old version is then stale; `add` creates a name that `list` and the worker's `query` see; a dotted `name` is refused;
- **stored queries:** parameterised text is refused by `query`; `save_query` refuses a function; a saved `users_of(t::Id)` runs on a read-only server by name with a name path and with a hex id (same rows); wrong count, wrong sort, a missing name and a missing query are errors;
- **review findings** (`review_findings_b063`): a relation's name path and its hex id give the same `run` rows; `add`'s version is the one `show` gives and `edit` accepts; text defining `bar` under `name: foo` answers `sample.bar`;
- **removals:** `rename` moves the name (the old one is gone); `delete` removes it; a second delete is an error; the number of definitions is unchanged;
- **errors:** six bad read calls are tool errors and the server answers after them; with `--max-memory 0` every query fails with the bound's error while `show` still works; a missing store file exits 1, a missing `--store` exits 2.

`tests/review_mistral.rs`, Mistral's probes (7): a read-only session writes nothing (only SQLite's side files appear, the log empty); a disallowed tool is not dispatched; flag-like arguments are refused, not honoured; `run` cannot execute a definition that is not a query; `delete` on a namespace path is refused; two edits with one version: exactly one wins; a read-only server runs stored queries without touching the store.

`cargo test -p sophia-mcp --all-features`: 24 passed (2026-11-18).

## Reviews

Triaged in [[Review 2026-11-18 sophia-mcp (external)]]: Codex on B-062, B-063 and B-064 (six defects, all fixed), Mistral on the permission check (no defect; one observation documented).
