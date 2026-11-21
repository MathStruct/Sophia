#planning

> Open backlog tasks: storage: measurements first, layout changes only with Daniel's agreement. Part of [[Backlog]], whose rules (rows, picking, closing) apply.

> Sources: [[Backlog]], [[Status]].

## Storage (measure first, layout changes need Daniel, SR-126)

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-034 | Compact local keys for reference edges (42 % of set.mm's store; measured 2026-11-20: 205 → 32 MiB, item 1 of the proposal in [[Lean Layout Measurements]]) | proposed | Daniel | M | SR-132; [[SP-011 Reference edges dominate storage on real code]] | measured gain; changed only with Daniel's agreement |
| B-036 | B-02's flat-layout comparison | planned | a flat layout | S | [[Milestones]] M0 | B-02 run on both layouts |
| B-096 | Build the per-user fact cache (SPEC-001 §8a): the cache store file, `facts/cache-policy` with the default `threshold(1 ms)` (SR-160), lookups before a tool runs | proposed | Daniel: the new store file (SR-126) | M | SR-152, SR-160; B-035, B-094, B-095; [[Fact Cache Experiment]] | Daniel agreed to the store file; the cache built with tests for each policy value; [[SPEC-001 Sophia Store v0]] §8a updated |
