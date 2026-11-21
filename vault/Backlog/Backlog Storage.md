#planning

> Open backlog tasks: storage: measurements first, layout changes only with Daniel's agreement. Part of [[Backlog]], whose rules (rows, picking, closing) apply.

> Sources: [[Backlog]], [[Status]].

## Storage (measure first, layout changes need Daniel, SR-126)

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-034 | Compact local keys for reference edges (42 % of set.mm's store; measured 2026-11-20: 205 → 32 MiB, item 1 of the proposal in [[Lean Layout Measurements]]) | proposed | Daniel | M | SR-132; [[SP-011 Reference edges dominate storage on real code]] | measured gain; changed only with Daniel's agreement |
| B-094 | B-035 part 1b: the fact cache's hit rates with Julia ids computed module-wide, as Moonkale's folder store computes them (`cid::julia::module`), instead of per file | planned | — | S | B-035; [[Review 2026-11-20 Codex B-035 (external)]] point 1 | `fact_cache` has a module-wide Julia mode; the Julia rows of [[Fact Cache Experiment]] re-measured in it, and the difference to per-file ids stated |
| B-095 | The fact cache's default policy and its store file: Daniel decides on `facts/cache-policy = threshold(1 ms)` ([[Fact Cache Experiment]], proposal) and on adding the per-user cache store (SR-126) | proposed | Daniel | S | SR-152, SR-126; B-035 | Daniel's decision recorded in [[SPEC-001 Sophia Store v0]] §8a; if accepted, a row for building the cache |
| B-036 | B-02's flat-layout comparison | planned | a flat layout | S | [[Milestones]] M0 | B-02 run on both layouts |
