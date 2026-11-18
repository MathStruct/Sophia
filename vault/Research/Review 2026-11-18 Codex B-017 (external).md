#reference

> A correctness review of B-017 (the hard memory bound: the query worker process under `RLIMIT_DATA`, the read-only opens) by Codex (GPT) on 2026-11-18, in the main checkout with probe tests allowed. Its sandbox could not reach the Nix daemon, so it ran nothing; Claude ran its probes. All three findings were right and are fixed, each covered by `review_findings` in `tests/bounded.rs`.
>
> Sources: the brief and the answer in `meta/scratchpad/b017/`. code: `crates/sophia-query/src/bounded.rs`, `tests/bounded.rs`.

## How it was run

```sh
codex exec -C ~/Code/CategoryTheory/Sophia -s workspace-write -o REVIEW-from-codex.md - < review-brief.md
```

## Findings

| # | finding | verdict | what was done |
|---|---|---|---|
| 1 | a request larger than the bound makes the worker's `read_to_end` fail (a fallible allocation), exit 2, and come back as `Worker` instead of `MemoryBound` | **right**: an 8 MiB text parameter under bounds of 2 to 12 MiB gave `Worker("… out of memory")` | the worker answers `MemoryBound` when reading the request fails with `OutOfMemory` |
| 2 | `Limits::deadline = Some(Duration::MAX)` panics in the caller (`Instant` addition overflows) after the worker started, and the worker is left unreaped | **right**: Codex's probe panicked at `bounded.rs:108` | checked addition; a deadline too far to represent waits for the worker, whose executor keeps its own deadline |
| 3 | a store path that is not UTF-8 is sent as raw bytes but decoded lossily, so the worker opens another path | **right**: an existing `store-\xff.sqlite` gave "unable to open database file" | the worker rebuilds the path from its bytes (`OsStrExt::from_bytes`) |

Codex's sweep of bounds from 0 to 16 MiB on an empty store: under 1 MiB the worker fails with `MemoryBound` (it cannot start), from 1 MiB it answers. So a bound below about 1 MiB refuses every query; noted in [[bounded]].
