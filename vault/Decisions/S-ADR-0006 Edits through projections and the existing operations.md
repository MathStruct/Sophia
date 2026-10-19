#design

> **Status: proposed (2026-10-17, Claude).** Related: [[SPEC-001 Sophia Store v0]] §6, §12.3; Moonkale ADR-0028, `packages/core/src/source/transaction.rs`; [[SP-004 Moonkale operations are file-shaped]].

## Context

Moonkale's `Op` set is file-shaped: `WriteText`, `CreateText`, `CreateDir`, `Rename` and `Delete`. Sophia's truth is a graph of immutable definitions under movable names. Moonkale asked for structural edits that are revision-checked and return the new content ids.

## Decision

In v0, a definition is edited as **text through a projection**, using the existing operations:
- `fetch_text(name)` prints;
- `WriteText` on a name parses, stores and moves the name, and is refused if the name's `seq` is not the expected `Version`;
- `CreateText` adds a definition;
- `Rename` and `Delete` move or remove names;
- `CreateDir` makes a namespace.

Structural operations (filling a hole, applying a rewrite, adding an annotation) come later, as new `Op` variants that Moonkale defines.

## Consequences

- WP-68 needs no change to Moonkale's core types.
- Every text edit goes through the parser, so the round-trip property (SPEC-005 §5) is load-bearing.
- A typo that fails to parse is a diagnostic, and nothing is stored. This is like a scratch file in Unison, except that the scratch is the editor buffer.
