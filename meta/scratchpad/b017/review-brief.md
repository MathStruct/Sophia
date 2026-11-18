# Review: B-017, a hard memory bound for untrusted queries (Sophia, Rust)

Repository: the current directory (~/Code/CategoryTheory/Sophia). The working tree has uncommitted changes from an earlier, already-reviewed task (B-014: crates/sophia-query/src/eval.rs, plan.rs, sophia-bench); ignore those.

## The requirement

SR-108: queries from untrusted agents get a hard memory bound enforced from outside the executor. Done means: a query that exceeds the bound fails cleanly (the caller survives and gets an error) on every backend (SQLite, redb, LMDB, DuckDB), tested.
Chosen design: run the query in a worker process (`sophia-query-worker`) that sets RLIMIT_DATA on itself; the caller maps the worker's outcome to `Error::MemoryBound`. Linux only. The worker opens the store read-only.

## The change

- Diff of most files: /tmp/claude-1005/-home-daniel-Code-CategoryTheory-Sophia/93aa2ff4-b9ac-4812-8e7a-932c74de893a/scratchpad/b017.diff
- Also: `Sqlite::open_read_only` in crates/sophia-store/src/sqlite.rs (search for it), and the new `Error::MemoryBound` / `Error::Worker` variants plus `pub mod bounded` in crates/sophia-query/src/sophia_query.rs.
- Note: crates/sophia-query/src/bounded.md describes intent.

## What to look for

Defects only: wrong results, panics, missing checks, ways a query can exceed the bound without a clean MemoryBound (or harm the caller), ways the caller can hang or deadlock (pipes, waits), protocol encode/decode mismatches, a read-only open that can still write or create, wrong mapping of signals/exit codes, races, missing tests. Not style.

## Rules

- Do not modify any tracked file. You may add probe tests as new files named crates/sophia-query/tests/codex_probe_*.rs and run them with `nix develop -c cargo test -p sophia-query --all-features --test <name>` (build output: send it to a file and grep it). Do not run the whole workspace.
- A finding is a claim until a probe shows it: mark each finding as VERIFIED (with the probe file and its output line) or UNVERIFIED.
- Answer: a bounded list, most severe first, each with file:line, the failure scenario, and VERIFIED/UNVERIFIED. No restated context.
