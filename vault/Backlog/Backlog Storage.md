#planning

> Open backlog tasks: storage: measurements first, layout changes only with Daniel's agreement. Part of [[Backlog]], whose rules (rows, picking, closing) apply.

> Sources: [[Backlog]], [[Status]].

## Storage (measure first, layout changes need Daniel, SR-126)

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-034 | Compact local keys for reference edges (42 % of set.mm's store; measured 2026-11-20: 205 → 32 MiB, item 1 of the proposal in [[Lean Layout Measurements]]) | proposed | Daniel | M | SR-132; [[SP-011 Reference edges dominate storage on real code]] | measured gain; changed only with Daniel's agreement |
| B-035 | The fact cache experiment: what enters it (every content-only fact, a cost threshold, off), default chosen by measurement | partial | — | M | SR-152; [[SPEC-001 Sophia Store v0]] §8a | the experiment's result and a default proposed to Daniel. **Part 1 done 2026-11-20** ([[Fact Cache Experiment]]): hits along histories and in fresh worktrees, size, lookup. Reviewed by Codex ([[Review 2026-11-20 Codex B-035 (external)]]); re-run pending. **Open:** part 1b, Julia ids module-wide as Moonkale computes them; part 2, Lean elaboration costs per declaration (idle machine), then re-time part 1 and set the threshold |
| B-036 | B-02's flat-layout comparison | planned | a flat layout | S | [[Milestones]] M0 | B-02 run on both layouts |
