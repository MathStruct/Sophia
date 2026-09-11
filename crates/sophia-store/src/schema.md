# schema

Node and edge kinds, their storage mapping, and migrations. Source: `schema.rs` (comments only).

Full design: [[Graph Schema]].

## The encapsulation rule

This file is the only place that should know the concrete labels. Every other module goes through these types, so that the schema really is replaceable — which is what [[Start Here]] explicitly invites ("all he needs to do is rewrite the core of the graph compiler and migrate the database to his own preferred schema"). An invitation like that is only credible if the code is arranged to honour it.

## Four groups of node kinds

Core (`Term`, `Type`, `Decl`, …), surface (`SurfaceNode`, `Span`, `Frontend`), IR (`Op`/`Region`/`Block` for [[MLIR Operation|MLIR]], `Instr`/`Func`/`Module` for [[LLVM IR]], plus `Target`), and the **knowledge layer** — `Witness`, `Prop`, `Test`, `Bench`, `Doc`. The last group is what [[Start Here]] calls annotations: statements in the database that are not there for compilation. See [[Tests and Documentation as Nodes]].

`Method` and `DispatchFact` are Julia-specific and exist for [[Content-Addressed Precompilation]]'s exact-invalidation claim; see [[World Age]].

## Edges are hashed

`h(edge) = H(ver ‖ "EDGE" ‖ kind ‖ h(src) ‖ h(dst) ‖ ord ‖ attrs)`. A [[Equivalence and Witnesses|witness]] must cite the specific `EQUIV` edge it justifies, and an unidentified edge cannot be cited. This is why [[merkle]] exposes `hash_edge`.

## The EQUIV level ordering, and the line in the middle

`Alpha`, `DefEq`, `Rewrite`, `Observational` are proofs. `Tested` and `Asserted` are evidence. **The line between them is the line between substitutable and not**, and it is drawn where it is because evidence is not a congruence: two sort functions agreeing on every test may differ on stability, and a context observing stability distinguishes them ([[Contextual Equivalence]]).

Silently substituting at the `Tested` or `Asserted` level is the most plausible route to this system producing a wrong program, which is why the enum comment says so and why the policy check lives at query time rather than being left to callers.

## Storage is two tables

`node`, `edge`, plus a `name` table. That is the whole relational mapping. Its simplicity is the argument for starting on DuckDB rather than a graph server — the hot queries are hash lookups and bounded traversals, both of which a columnar engine does very well. See [[Query Cookbook]] and [[Property Graph]].

## Migrations

A hash schema change re-hashes everything. Supported explicitly: version-first hashing gives disjoint hash spaces, versions coexist, `MigratesTo` relates identities, and migration is **re-elaboration from core terms** rather than a text rewrite. That is why it is cheap, and it is the concrete reason core terms — not source text and not IR — are the source of truth.

## Related

- [[Graph Schema]] · [[sophia_store]] · [[Query Cookbook]]
- [[Equivalence and Witnesses]] · [[Tests and Documentation as Nodes]]
- [[Hashing and Identity]] · [[merkle]]
