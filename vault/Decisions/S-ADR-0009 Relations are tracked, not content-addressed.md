#design

> **Status: proposed (2026-10-20, Claude), after Daniel asked "should they have a type or be tracked?"** Related: [[SPEC-003 Sophialang Core v0]] §1, [[SP-006 Open method tables versus content addressing]], `crates/sophialang-core/src/sophialang_core.md`.

## Context

In the sample program, `parent` and `ancestor` were both relations `Nat → Nat → Prop` with no body. Because a relation's identity was the hash of its content (its kind and type), they became **one** definition, with two names.

A relation is defined by its rules, and rules are added over time, just as methods are added to a generic function. Hashing the relation's content cannot tell apart two relations that have the same type, and hashing "the relation with its rules" would change every caller's id whenever a rule is added. That is the invalidation storm of SP-006 again.

## Options

1. **Content-addressed** (as now): a relation *is* its type. Equal types merge.
2. **Content-addressed with its rules**: every new rule gives the relation, and everything that uses it, a new id.
3. **Tracked (nominal)**: a relation gets a fresh identity, like a struct or a generic function, and its declared type is stored with it. Rules join it through a versioned relation, exactly like methods join a generic function.

## Proposal

Option 3. A relation keeps a type, but the type is *about* it, not its identity. Rules stay content-addressed (each rule is fixed content) and are attached to their relation through the store's versioned nominal relation. Query results that relied on the rule set record its version, and become stale when a rule is added or removed (SPEC-001 §8).

## Consequences

- In SPEC-003 §1, `relation` moves from the content-addressed kinds to the nominal kinds. A rule's head refers to its relation by nominal id.
- This is symmetric with multiple dispatch: generic function ↔ relation, method ↔ rule.
- To be done in `sophialang-core` and the sample program once Daniel confirms.
