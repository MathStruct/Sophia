# egraph

E-graph construction, saturation and extraction. Source: `egraph.rs` (comments only).

Background: [[E-Graph]], [[Equality Saturation]]. How it fits the compiler: [[Compilation as Query]].

## A thin layer, deliberately

`egg` exists, is excellent, and Cranelift now ships e-graph-based optimisation in production ([[State of the Art - Sea of Nodes and Graph IRs]]). This is one of the *lower*-risk parts of the project and should be integration work, not research.

## The correspondence that makes this the right structure

| e-graph concept | Sophia concept |
| --- | --- |
| e-class | equivalence class of hashes |
| congruence closure | the requirement that `EQUIV` be a congruence |
| proof extraction | the rewrite-chain witness format |

The match is close enough that the e-graph is not a technique the equivalence layer happens to use — it is the operational meaning of the equivalence layer.

## `writeback` is the point

An ordinary compiler discovers equivalences during a build and throws them away. `writeback` persists them, so they are available to every later build on any machine. That accumulation across builds and across users is the strongest argument for the whole architecture, and it costs nothing beyond having a store.

## Rules are nodes

A rule is stored, versioned, hashed and citable. A rewrite-chain [[witness]] cites rule hashes, so the soundness of a chain reduces to the soundness of its rules — which are either themselves witnessed or explicitly marked as signed axioms. This is what keeps the [[Trusted Computing Base]] enumerable as the rule set grows.

## Rules must be effect-guarded

Nearly every interesting rewrite is sound only on effect-free subterms: dropping an unused `let` requires the bound expression to be pure. The guard consults the effect row from [[effects]]. **A rule set without effect guards is a miscompilation generator**, and this is the most likely place for that mistake to be made, because the rules look correct in isolation.

## Extraction and budgets

Greedy bottom-up with a cycle check by default; ILP only for small hot regions, since the acyclicity constraint makes optimal extraction NP-hard. The cost model comes from the `Target` node and — per STOKE's results ([[State of the Art - Superoptimization and Synthesis]]) — matters more than the search does.

Budgets are mandatory: associativity and commutativity blow e-graphs up, nothing saturates to a true fixpoint in practice, and the report should record *which* budget was hit so a build that silently got a worse result says so.

## Calibrated expectations

It will find peepholes, algebraic simplification, strength reduction, constant folding. It will **not** discover that a Julia sort and a C++ sort are equivalent — superoptimisers run out of room at a handful of operations and this is the same search. Expecting otherwise is the main way this component would disappoint.

## Related

- [[E-Graph]] · [[Equality Saturation]] · [[Term Rewriting System]]
- [[sophia_equiv]] · [[witness]] · [[Compilation as Query]]
- [[State of the Art - Equality Saturation and E-Graphs]] · [[State of the Art - Superoptimization and Synthesis]]
