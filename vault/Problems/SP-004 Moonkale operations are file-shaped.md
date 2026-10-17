#open-problem

> **Status: open, later (2026-10-17).** Related: [[S-ADR-0006 Edits through projections and the existing operations]]; Moonkale `packages/core/src/source/transaction.rs`.

**Problem.** Moonkale's `Op` set (`WriteText`, `CreateText`, `CreateDir`, `Rename`, `Delete`) covers v0 edits through text. It has no operation for:
- adding or removing an annotation;
- filling a hole with a term;
- applying a rewrite chosen in an e-graph view;
- re-pointing a name at an existing content id (for example, a revert along a lineage).

**Who decides.** Moonkale owns `moonkale-core` and its CHANGELOG. Sophia proposes the operations it needs once they are needed:
- `Annotate { subjects, kind, payload }`: the Moonkale agent leans towards a real `Op::Annotate` when its WP-19 lands, rather than pseudo-text nodes (review of 2026-10-17);
- `SetName { name, target, expected }`;
- `ApplyTerm { name, path, term, expected }`.
