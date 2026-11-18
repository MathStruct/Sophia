#implementation

> The native executor: semi-naive bottom-up evaluation over any backend's `ReadTx`. Source: `eval.rs`.
>
> Sources: Ullman, *Principles of Database and Knowledge-Base Systems* (semi-naive evaluation); [[SPEC-007 Sophia Query Language]] §6. code: `eval.rs`

- Strata run in dependency order. In a recursive stratum, each round evaluates every rule once per recursive literal, with that literal reading only the previous round's new tuples (the delta).
- Base literals read through access paths (`kind_of`, `records_of_kind`, `refs_of`, `dependants`, `name`, `names_of`, `relation`, `nominal`, `annotations_of`, `facts_of`), memoised per run, or through a full scan (`all_kinds`, `all_refs`, …) when no column is bound.
- Local relations keep hash indexes on the column sets the plans look them up by.
- `Limits` is the resource budget: join steps, rows, derived tuples, **estimated bytes**, a wall-clock **deadline**, and a cancel flag (`Exec::with_cancel`). The byte estimate counts the tuples the executor keeps (lists, sets, indexes, base memo and scans, select rows); it is not an allocator count. The derived-tuple cap is a logical limit. The step budget alone is not one: a closure of 10⁵ definitions grew past 14 GB before reaching 50M steps. Each round's new tuples are deduplicated against everything known as they are produced, so duplicates never pile up. Without counts, the select stops one row after the row cap and marks the result truncated.
- `Stats` count lookups, scans, derived tuples and rounds (reported by the CLI and the benchmarks).
- `count` with no grouping columns over no solutions gives one row of zeros, as SQL does.

**Set-oriented access (B-014, 2026-11-18).** Three costs the tuple-at-a-time executor paid on whole-relation queries ([[Benchmark Results]], B-17):
- **A prefix access path.** `name(p, _)` with a bound `startswith(p, s)` reads only the names that start with `s`: pages of `names_from` from `s`, under the namespace part of `s` (up to its last `.`), stopping at the first path past the range. Any string is a prefix, not only a namespace. Memoised per prefix; the `startswith` literal still runs as a filter. The planner marks the step (`plan::name_prefix`).
- **Anti-joins.** A negated base literal is probed once per solution. After `Tuning::anti_join_after` probes (256) on one column set, the executor scans the relation once, keeps the projection on the bound columns as a hash set, and probes that. Only when the literal's unbound arguments are distinct variables: `!refs(x, x)` with `x` unbound asks for a self-reference, which a projection cannot see.
- **Borrowed reads.** Terms are read by reference (`val`), not cloned; `unify` records bindings on one trail instead of a `Vec` per tuple; local indexes on one column are probed without building a key. `count` groups solutions as they arrive (the distinct counted values per group), without first collecting and de-duplicating rows; groups plus distinct counted values are bounded by `max_derived`.

`Tuning` (and `Prepared::run_tuned`) choose these; `Tuning::TUPLE_AT_A_TIME` turns the prefix path and anti-joins off, for tests and benchmarks. The rows never depend on it (`tests/set_oriented.rs` compares three tunings on every backend, and the SQL translation). `Stats` count `prefix_reads` and `anti_joins`.

**The byte count is shared (2026-11-18).** The estimated bytes live in one counter shared with the closures that receive a join's solutions. Before, the select and each round copied the count, and wrote the copy back after the join: base tuples read in the middle of a join were dropped from the count and the budget (a select over a scan reported 3 952 bytes for two full scans; regression test `bytes_count_base_reads_inside_joins`).

**No clock on wasm (2026-10-24).** `wasm32-unknown-unknown` has no clock in `std` (`Instant::now` panics), so there the query deadline is not enforced (`clock()` returns `None`); the step and row budgets still bound a query. A browser clock can replace that placeholder.
