#reference #open-problem

> A cross-repository review of Moonkale (`4b914c8c`) and Sophia (`5758f0a`), written by Fable on 2026-11-10. Daniel placed it beside the Moonkale folders: `~/Code/Moonkale/Moonkale-Sophia-Review-2026-11-10/`, outside both repositories. It has seven parts, a `repro/` crate per repository (a test passes while the reported behaviour is present), and logs of the runs. By its own account it read about a quarter of Sophia's code line by line (`sophia-hash`, most of `sophia-store`, all of `sophia-interface`), and it did not read `sophia-query`, `sophialang-*`, `sophia-ingest` or the KV and DuckDB backends. This note records each Sophia point and whether it was right on `5758f0a`. Moonkale's points (A-1, A-2, C-*, M-*, D-8 to D-13) are the Moonkale agent's to triage. Triaged 2026-11-10 by Claude (Sophia). **Checked by reading Sophia's code, not by running the repro crate:** Claude Code's auto mode refused to run code from outside the repository.
>
> Sources: the review's files `README.md`, `01_…` to `07_…`, `repro/sophia-repro/tests/findings.rs`. code: the files named below.

## Bugs and trust (01, 02 B)

| ID | claim | verdict | evidence |
|---|---|---|---|
| S-1 / B-1 | `put_new` stores a record whatever its bytes; `verify` finds it afterwards | **right** | `logic.rs:162`: no hash check on write. For sophialang records a check costs one hash per component. For Moonkale's records Sophia cannot check (SPEC-009 §1), so "content-addressed" holds only for honest writers there. **A decision** |
| S-2 | `changes_since(_, usize::MAX)` overflows `limit + 1` | **right** | `logic.rs:525`. The same bug as the DuckDB row cap fixed in `v0.1.10` |
| S-3 / A-3 | the read-only SQL guard bounds steps and rows, not memory: one `zeroblob(300000000)` row took ~0.9 GB | **right** (the code has no memory limit; the 0.9 GB is the reviewer's measurement) | `sqlite.rs` sets a progress handler and an authorizer, but no `SQLITE_LIMIT_LENGTH` or heap limit |
| S-4 | one file may name two records with one name; the last wins | **right** | `folder.rs:401-405` fills a map; `validate_batch` dedups names with a set (`:982`). Note: a top-level rebinding in Julia (`x = 1` … `x = 2`) is legal, and in Julia too the last wins. Before refusing it, check what Moonkale's normaliser emits for such a file |
| S-5 | `path_from_moonkale` maps `a.b/c` and `a/b/c` to one path; a backquoted segment with `/` or `.` breaks the reverse mapping | **right** | `sophia_interface.rs:126-132` |
| S-7 / B-2 | `set_settings` writes any meta key, `store/mode` included | **right** | `logic.rs:606`. Every caller uses a fixed prefix today, so this is an API hazard |
| S-8 | reference counts read a malformed value as 0 and clamp an underflow | **right** | `folder.rs:423, 590, 648` |
| S-9 | deleting a namespace leaves its sub-namespace markers; it is refused while a name equal to the namespace exists | not checked | `edit.rs:411` |
| S-10 | `create_text` returns the first definition's key when the text does not define `name` | **right, documented** | `edit.rs:357` says so ("returns the key of `name` if the text defines it"). Whether to refuse instead is a choice for Moonkale |
| S-11 | a removed fact reaches subscribers as a change with no keys | not checked | |
| S-12 | batch validation errors are `NotFound` | **right** | `folder.rs:939-958` |
| S-13 | `Writer` writes lengths `as u32`, which truncates past 4 GiB | **right** (far from today's sizes) | `sophia-hash/src/canonical.rs` |
| S-14 | `write_settings` reads the version and writes `found + 1` in separate transactions | **right** | `edit.rs:421-437` |
| S-15 | a rename is logged as remove and add | **right, known** | SP-004 (operations are file-shaped) |
| SP-008 | `apply` spans several transactions | **right, known** | [[SP-008 Edits span several transactions]]. The review's acceptance test, a backend that fails the n-th write, is a good one |

## Cost (03)

I-1 to I-9 are by reading. Two were checked:
- **I-7** (`backend.rs` default methods that every backend overrides) is a fair trap for a fifth backend.
- **I-8**, worlds stored under `world/` and never removed, is **right** (`logic.rs:577`).

None was measured, so none is changed before a benchmark shows it matters (SR-126).

## Tests, gates, vault (04, 05)

| ID | claim | verdict |
|---|---|---|
| 04.1, G-5 | Sophia has no CI that runs the tests; only `deploy.yml` | **right** |
| 04.2 | `cargo fmt --check` differs in 81 files; no `rustfmt.toml` | not rechecked; plausible (the code is written ~140 wide) |
| 04.3 | one clippy warning, `tests/folder.rs:195` | **right**, the known one |
| 04.5 | thin: `sophia-cli` has no test; the guard is not tested for resources; no test for S-1, S-2, S-4 | **right** |
| D-1 | two wikilinks broken by a line break, in RQ-001 and RQ-003 | **right** |
| D-2 | Status says "eight drafts"; there are ten specifications | **right** |
| D-3 | Status is a history, not a current state | **right**. Daniel's call how to restructure |
| D-4 | the README undersells `sophia-ingest` (Julia, Rust, Python) | not rechecked |
| D-7 | `attic/` is tracked and not built | **right, intended**: AGENTS.md lists it as "kept for reference, not built" |

## Structure and the boundary (06, 07)

- **X-1:** the dependency between the repositories is circular: Moonkale pins Sophia by tag, and `sophia-ingest` pins `moonkale-content-id` by revision. The review proposes moving the normaliser to its own repository, or into Sophia, so that the dependency runs one way and Sophia can check every record. **A decision for Daniel and Moonkale.**
- **X-2:** the store's verification rule names a Moonkale work package (`moonkale-wp66`). That is **right**, and a registry of foreign normalisers would remove it.
- **X-3:** in a store-truth folder (SPEC-010) the store holds the only copy, under ids Sophia cannot recompute. That is **right**, and it is the strongest argument for X-1.
- **07 §5, a consumer test before tagging:** run Moonkale's fixtures through `store_code` and `verify` before tagging. That is **right**. `v0.1.10` would have failed it.
- **07 §3, identity:** automorphic members are one definition, but bisimilar ones are not (`f = g, g = g`). That is **right**, and it is the content of [[RQ-001 Recursive Groups up to Bisimilarity]]. The review proposes adding its two tests there as the statement of what v0 does.
- **06, duplicates:** `split_path` exists in `sophia-store` and in `sophialang-syntax`, which is **right**. The SQL backends share about 105 lines; the review itself advises leaving them.
- **Refactoring proposals** (`_in` forms for SP-008, typed meta keys, a counter type, `InvalidBatch`): these are proposals, not findings.
- **Directions** (Sophia as a service, the change log as history and blame, queries as the agent interface): these are proposals for Daniel.

## Earlier review

Its table of the 2026-11-07 findings matches [[Review 2026-11-07 Gemini (external)]].

## Done (2026-11-10, `v0.1.12`, on Daniel's go: "do the small fixes & gates")

Each fix has a test that fails without it, checked by undoing the fix:
- **S-2:** `changes_since` saturates (`settings_are_listed_and_logged`).
- **S-3:** SQLite's query connection has `SQLITE_LIMIT_LENGTH` (at most 16 MiB) and a result byte budget, and DuckDB runs user queries under a 512 MiB `memory_limit` with the same budget (`sql_queries_are_read_only_and_capped`, `duckdb_queries_are_read_only_capped_and_timed`). `QueryLimits` gained `max_bytes`.
- **S-7:** `set_settings` accepts only its four prefixes (`NotASetting`).
- **S-8:** counters fail on a wrong length or an underflow (`batches_and_counters_are_checked`).
- **S-12:** `InvalidBatch` replaces `NotFound` for malformed batches.
- **S-14:** `write_settings` checks its version in the write transaction (`Store::set_settings_if`).
- **S-13:** framing lengths panic at 4 GiB instead of truncating. This one has no test: it would need a 4 GiB value.
- **D-1, D-2:** the two links and the specification count.

The gates:
- **CI:** `.github/workflows/ci.yml` runs formatting, clippy with `-D warnings`, and all tests with all features. The wasm tests are not in it yet.
- **Clippy:** both warnings are fixed.
- **Formatting:** `rustfmt.toml` (`max_width = 140`, `use_small_heuristics = "Max"`), then one formatting commit.
- **The consumer test:** `moonkale_fixtures_verify_under_every_moonkale_normaliser`. It fails with `v0.1.10`'s rule.

## Still open

Decisions for Daniel:
- **S-1:** verify sophialang records on write.
- **S-4:** refuse a name bound twice in one file, or keep the last, as Julia does.
- **S-5:** escape `.` in Moonkale's path segments, or forbid it.
- **X-1, X-2:** where the normaliser lives, and a registry of foreign normalisers instead of the `moonkale-wp66` prefix.
- **SP-008:** with the review's failing-backend acceptance test.

Not done: a heap limit for SQLite (`sqlite3_hard_heap_limit64` is process-wide), the wasm tests in CI, and the cost points (I-1 to I-9: measure first).
