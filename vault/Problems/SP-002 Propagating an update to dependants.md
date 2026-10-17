#open-problem

> **Status: open, later (2026-10-17).** Related: [[SPEC-001 Base Sophia v0]] §5.1; [[Naming and Change Propagation]]; Unison's `update`, which propagates.

**Problem.** References are by content (S-ADR-0002). Editing `f` creates `f'` and moves the name `f`, but `g`, which calls `f`, still refers to the old `f`. Unison's `update` re-elaborates the dependants against the new version and moves their names too, provided they still typecheck.

**For v0.** No propagation. The store reports the dependants that still use the old content, and Moonkale can show them as "uses an old version".

**Later.** Propagation is a batch edit:
- walk the transitive dependants in dependency order;
- re-elaborate each one with `f ↦ f'`;
- typecheck it once a checker exists;
- move its names;
- extend its lineage.

Dependants that fail become a visible list of goals, not a half-applied update.
