#design

> **Status: proposed (2026-10-20, Claude).** It answers the review's question ([[Review 2026-10-20 Query Language (external)]]): should a query be a special sophialang DSL node, a separate IR sharing common primitives, or both? Related: [[SPEC-007 Sophia Query Language]] §3.4, §9; [[SPEC-003 Sophialang Core v0]]; SR-97, SR-99.

## Context

sophialang's core is a typed λ-calculus with call-by-value evaluation (SPEC-003). A query is a set of Horn clauses whose meaning is a least fixpoint. The two have different evaluation semantics: `x == y` is a test in one and a constraint in the other. They share a great deal of infrastructure:
- canonical framing;
- the identity rule;
- strongly connected components;
- records, names, lineage;
- rendering profiles.

## Options

1. **A DSL node in the core:** queries are sophialang terms, and the core evaluator gains relational semantics.
2. **A separate IR** with its own semantics, on the shared primitives.
3. **Both:** a separate IR now, and later a quoted core node that holds a query as a value.

## Proposal

Option 3, in that order.
- **Now:** `sophia-query` is a separate language in the store (`lang = sophia-query`). It reuses `sophia-hash` (framing, digests, Tarjan) and the store's record model. The core stays small, and its evaluator does not learn fixpoints.
- **Later (with M4, the interpreter):** a core node `Query{S, A}` whose payload is the query IR, treated as an opaque, typed value. sophialang code builds, composes and runs queries through primitives (`run(q, args)`). The evaluator calls the query executor; it never reinterprets sophialang expressions relationally (SR-99).
- **Self-hosting:** the query implementation is rewritten in sophialang once the interpreter exists (M6). The IR does not change, so stored query ids survive.

## Consequences

- The query IR's canonical form and ids are independent of the core's normaliser version. A core change does not rename every stored query, and the reverse holds too.
- A cross-IR optimisation ("partial evaluation of a function against a query result") needs both IRs in one rewrite system. That is the e-graph research direction (SR-106), not a v0 concern.
