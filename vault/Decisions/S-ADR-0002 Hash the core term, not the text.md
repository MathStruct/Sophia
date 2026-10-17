#design

> **Status: proposed (2026-10-17, Claude).** Answers Moonkale question 1a. Related: [[SPEC-001 Base Sophia v0]] §4, [[Theory of the v0 Store]], Moonkale Code Graph Schema §4 and its WP-66 section.

## Context

Moonkale needs to know what a Sophia content id is a hash *of*. There are three candidates:
- the surface text;
- a concrete syntax tree;
- the elaborated core term.

Sophia has no files (SR-01), and its text is meant to be each reader's choice of projection (SR-19).

## Options

1. **Surface text or a concrete syntax tree.** Simple. But two readers' projections would give two ids for one program, which defeats SR-19.
2. **The core term, types included.** The identity is the meaning as the core sees it. In a dependent core the declared type is part of that meaning.
3. **The core term without types.** Two definitions that differ only in type, such as `Int → Int` and `∀A. A → A`, would collide.

## Decision

Option 2:
- BLAKE3 over a tag-length-value canonical encoding of the core term and the declared type;
- tagged with `sophia-core` and a hash version;
- locals as de Bruijn indices; own name excluded; references by the referenced digest;
- recursive groups ordered by Moonkale's WP-66 v1 algorithm.

Not hashed: inferred types and other systems' typings, docs, proofs, tests, binder display names.

## Consequences

- A projection change never changes an id.
- Every change to a dependency changes the dependant's id. Updating dependants is a separate feature ([[SP-002 Propagating an update to dependants]]).
- α-equivalent definitions are one definition, with one set of display names ([[SP-003 Display names of alpha-equivalent definitions]]).
- Changing the core changes the hash version, and a migration writes `migrated_to` (SPEC-001 §4.5).
