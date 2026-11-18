# B-064 review

## Verified findings

1. **[P2] `facts` does not cap its nested lists.** `crates/sophia-mcp/src/sophia_mcp.rs:418` (inputs), `:437` (annotation `with`), `:447` (truncation flag). With `max_nodes=1`, one fact containing two inputs and one annotation containing two other subjects returns both two-item lists and `truncated=false`. This violates requirement 5's per-list bound and permits arbitrarily large responses despite the configured cap. Reproduce: `CARGO_TARGET_DIR="$PWD/target/probe-extra" cargo run --manifest-path scratch/probe/Cargo.toml --offline`. Evidence: `scratch/probe-extra.log`, `scratch/caps-summary.log`: `max_nodes=1 facts=1 annotations=1 inputs=2 with=2 truncated=False`. Add regression coverage for nested and nonzero outer-list truncation; the submitted tests only assert `truncated=false`.

2. **[P2] Startup can create a store from an empty SQLite file.** `crates/sophia-mcp/src/backends.rs:65` accepts any existing file; `:90` uses the creating/initialising `Sqlite::open`. With `--allow-edits`, a zero-byte placeholder becomes a new Sophia database, violating requirement 4. Without flags it is also accepted at startup, but tools then fail on missing tables. Reproduce: `python3 scratch/startup.py` (uses the server built in `target/probe/debug/`). Output: read-only `exit=0 size=0 initialized=True`, zero tables; edit-enabled `exit=0 size=94208 initialized=True`, 12 tables. See `scratch/startup.log`. This SQLite behavior **predates the patch**; it is an unmet stated requirement, not a newly introduced regression. Missing regression: reject an existing empty/non-Sophia database without initialising it.

No additional suspected defects are asserted. Probe records and validation results follow.

## Probe log

## Probe 1 — source inspection
Checked the complete tracked diff and both untracked Rust files, backend open functions, and worker dispatch. No execution finding yet. Commands: `git diff -- crates/sophia-mcp`, `cat crates/sophia-mcp/src/backends.rs crates/sophia-mcp/tests/backends.rs`, and source searches. The main checkout has no `HANDOFF-from-moonkale.md`.

## Probe 2 — all-feature test run
Command: `cargo test -p sophia-mcp --all-features > scratch/all-features.log 2>&1`. Running, with this worktree's own `target/`. Scratch stores in subsequent probes will live under `scratch/`.

## Probe 3 — startup and nested-list limits (running)
Reproducer: `scratch/probe/src/main.rs` (standalone Rust package in `scratch/probe/`). Command: `CARGO_TARGET_DIR="$PWD/target/probe" cargo run --manifest-path scratch/probe/Cargo.toml --offline > scratch/probe-isolated.log 2>&1`. Separate build directory under this worktree's target avoids waiting for the optional DuckDB build. Checks existing empty SQLite file with/without edits, and `max_nodes=1` with two fact inputs and two annotation co-subjects. No result yet.

## Probe 3 result — two verified findings
Reproducer passed (exit 0); full output: `scratch/probe-isolated.log`.

* **P2: nested lists exceed `max_nodes` and do not set `truncated`.** `crates/sophia-mcp/src/sophia_mcp.rs:418` and `:437` collect every input and every annotation co-subject. With `Config.max_nodes=1`, one fact with two inputs and one annotation with two other subjects returns `inputs` length 2, `with` length 2, and `"truncated":false`. Requirement 5 caps each list. Only the outer facts/annotations lists are capped; the current new tests never exercise truncation (their only truncated assertion is false).

* **P2: existing empty file creates a SQLite store with edits enabled; invalid store accepted without edits.** `crates/sophia-mcp/src/backends.rs:90` calls the creating/initialising `Sqlite::open`, after `Kind::exists` at `:65` checks only `is_file`. Probe output: `empty file writes=false: startup OK; len=0; facts=Err("backend error: no such table: t_name")`; `empty file writes=true: startup OK; len=4096; facts=Err("no name missing")`. Thus an empty placeholder is turned into a new Sophia database, contrary to requirement 4. The nonvalidating SQLite behavior predates this patch, but remains a failure of the stated requirement; there is no invalid/empty database regression test.

## Probe 4 — default tests and minimal-feature build (running)
Commands: `TMPDIR="$PWD/scratch/tmp" CARGO_TARGET_DIR="$PWD/target/probe" cargo test -p sophia-mcp > scratch/default-tests.log 2>&1`, then the same environment with `cargo build -p sophia-mcp --no-default-features > scratch/no-default.log 2>&1`. Stores from these tests use `scratch/tmp/`.

## Probe 5 — nominal paths, kind filter and zero cap (running)
Extended `scratch/probe/src/main.rs` to assert nominal name/hex equivalence, filtering, and zero-cap truncation. Command: `CARGO_TARGET_DIR="$PWD/target/probe-extra" cargo run --manifest-path scratch/probe/Cargo.toml --offline > scratch/probe-extra.log 2>&1`.

## Probe 4 interim result
Default-feature integration tests PASS: 4 backend tests, 7 prior review tests, 11 stdio tests (22 total), 0 failed. Full output `scratch/default-tests.log`. Minimal-feature build still running.

## Probe 6 — CLI startup on empty files
Script: `scratch/startup.py`; command: `python3 scratch/startup.py > scratch/startup.log 2>&1`. Checks the actual server executable, performs MCP initialization, and counts SQLite tables afterward. See log for result.

## Probe 6 result — confirmed via CLI
`flags=[] exit=0 size=0 initialized=True stderr=''`, `tables=0`.
`flags=['--allow-edits'] exit=0 size=94208 initialized=True stderr=''`, `tables=12`.
Both starts succeeded; the edit-enabled one created the schema and built-in profiles. This independently confirms Probe 3's startup finding through stdio, with successful process exit.

## Probe 5 result — PASS for nominal paths, filtering and zero cap
`scratch/probe-extra.log` ends with `nominal path equals hex and kind filter: PASS` and `cap=0 outer lists empty and truncated=true: PASS`. The nested-list finding is reproduced again immediately above those lines.

## Probe 7 — single-backend feature builds (running)
Commands: `CARGO_TARGET_DIR="$PWD/target/probe" cargo build -p sophia-mcp --no-default-features --features redb > scratch/redb-only.log 2>&1`, then equivalent `--features lmdb > scratch/lmdb-only.log 2>&1`. Checks cfg arms with only one optional default backend.

## Probe 4/7 build results — PASS
`--no-default-features`: exit 0, finished in 22.19s (`scratch/no-default.log`).
`--no-default-features --features redb`: exit 0, finished in 3.96s (`scratch/redb-only.log`).
`--no-default-features --features lmdb`: exit 0, finished in 4.34s (`scratch/lmdb-only.log`).

## Probe 2 result — all-feature tests PASS
`cargo test -p sophia-mcp --all-features` exited 0. All 22 integration tests passed, including all four backends, worker queries, unchanged data-file checks, write-flag refusal, and LMDB writes. Full log: `scratch/all-features.log`. The stock tests in this initial invocation used their default temporary-directory location; the later default-feature test run used `scratch/tmp/`.

## Probe 8 — remaining feature combinations (running)
Script: `scratch/duck-matrix.sh`; command: `bash scratch/duck-matrix.sh > scratch/duck-matrix.log 2>&1`. Builds DuckDB alone, redb+DuckDB, and LMDB+DuckDB with default features disabled. Combined with the previous builds/tests this covers all eight feature combinations.

## Probe 3/5 compact output
Extracted the returned JSON in `scratch/probe-extra.log` using Python's JSON parser: `scratch/caps-summary.log` says `max_nodes=1 facts=1 annotations=1 inputs=2 with=2 truncated=False`. This is the concrete nested-list limit violation.

## Initial queued scratch run result
The original queued command (`CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path scratch/probe/Cargo.toml --offline`, log `scratch/probe.log`) also finished with exit 0 and reproduced both findings. No conflicting results.

## Probe 8 interim result
DuckDB alone (`--no-default-features --features duckdb`) PASS, finished in 5m 48s. The other two mixed combinations are still building. Full compiler log `scratch/features-duckdb.log`.

## Probe 8 second result
redb+DuckDB PASS, finished in 20.58s (`scratch/features-redb-duckdb.log`). LMDB+DuckDB is rebuilding the native dependency for its build graph; no compiler error reported yet.

## Probe 8 final result — PASS for all remaining combinations
`scratch/duck-matrix.log`:
```
duckdb: PASS
redb,duckdb: PASS
lmdb,duckdb: PASS
```
LMDB+DuckDB finished in 7m 04s (`scratch/features-lmdb-duckdb.log`). All eight feature combinations built successfully (none, redb, lmdb, redb+lmdb/default, duckdb, redb+duckdb, lmdb+duckdb, all).

## Final workspace check
`du -sh target`: `26G target`, below 30 GB, so no cleanup required. All review builds finished. `git status --short` and `git diff --stat` show the original four tracked modifications and original untracked change files unchanged, plus `scratch/`. No commits or pushes. Review artifacts and stores are under `scratch/`; build artifacts are in this worktree's own `target/`.
