# Review: B-064, sophia-mcp on every backend and a `facts` tool

You are reviewing an uncommitted change in this worktree (`git diff` and the two untracked files; the same change is in `b064.diff` at the worktree root). Do not commit, push, or change files outside `scratch/` (create it). Use this worktree's own `target/`.

## Requirement

`sophia-mcp` (crates/sophia-mcp) is an MCP server on stdio over a Sophia store. Queries run in a separate worker process (`sophia-mcp-worker`, see `crates/sophia-query/src/bounded.rs`) that opens the store read-only.

1. The server must serve stores on four backends: SQLite, redb, LMDB, DuckDB, chosen by `--backend sqlite|redb|lmdb|duckdb` (default sqlite).
2. Without `--allow-edits`/`--allow-removals` the store is opened read-only and a session must not change the store's data.
3. redb and DuckDB lock their file against other processes while it is open for writing, so on those two backends the write flags must be refused at startup (exit 1, clear message) and nothing written. SQLite and LMDB serve every tool class.
4. The server never creates a store; a missing store is an error at startup (exit 1); a bad argument exits 2.
5. A new read tool `facts(path, kind?)`: `path` is a dotted name or a hex id; it returns the derived facts (`facts_of`) about the definition or nominal entity the name points to, each with its value, producer, inputs and whether it is stale (`Store::is_stale`), and the annotations on it (`annotations_of`), with the other subjects of each annotation in `with`. `kind` filters by kind. Answers are capped at `max_nodes` per list, with `truncated` set when cut. It must be listed and callable without any flag, and registered in `TOOLS` with class `Read`.
6. Feature flags: `redb` and `lmdb` on by default, `duckdb` optional; every combination must build.

## What to report

Defects only: wrong results, panics, missing checks, a way to write to a store that should be read-only, a way to create a store, races between the server and its worker, wrong `cfg` handling, tests that do not test what they claim, missing tests. For each: file and line, what goes wrong, and how to reproduce it. If you can, show it with a scratch test or command and its output. Mark verified findings apart from suspicions.

Useful commands: `cargo test -p sophia-mcp --all-features`, `cargo build -p sophia-mcp --no-default-features`. Note /tmp is a small RAM disk; put scratch stores under `scratch/`.

Write your answer to `scratch/ANSWER.md` as you go: after each probe, append one entry (what it checked, the script's path, passed or the finding with its output). Finish with the verified findings first.
