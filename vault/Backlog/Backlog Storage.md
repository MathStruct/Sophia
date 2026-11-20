#planning

> Open backlog tasks: storage: measurements first, layout changes only with Daniel's agreement. Part of [[Backlog]], whose rules (rows, picking, closing) apply.

> Sources: [[Backlog]], [[Status]].

## Storage (measure first, layout changes need Daniel, SR-126)

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-034 | Compact local keys for reference edges (42 % of set.mm's store; measured 2026-11-20: 205 → 32 MiB, item 1 of the proposal in [[Lean Layout Measurements]]) | proposed | Daniel | M | SR-132; [[SP-011 Reference edges dominate storage on real code]] | measured gain; changed only with Daniel's agreement |
| B-035 | The fact cache experiment: what enters it (every content-only fact, a cost threshold, off), default chosen by measurement | planned | — | M | SR-152; [[SPEC-001 Sophia Store v0]] §8a | the experiment's result and a default proposed to Daniel |
| B-036 | B-02's flat-layout comparison | planned | a flat layout | S | [[Milestones]] M0 | B-02 run on both layouts |
