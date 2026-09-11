# Roadmap

Milestones ordered so that each one produces something demonstrable, and so that the *riskiest assumptions are tested as early as they can be*. The ordering deliberately delays the headline goal (cross-language equivalence) because it depends on everything else being right.

## M0 — Schema and identity, proved out on the smallest possible language

**Target language: Metamath.** Chosen because its grammar fits on a page, it has no type inference, no effects and no memory model, and it already *is* a database of statements and proofs. Nothing about Metamath is representative of Julia, which is exactly why it is a good first test of the plumbing rather than of the semantics.

Deliverables:
- `node`/`edge` tables per [[Graph Schema]], on DuckDB.
- Canonicalisation + [[Hashing and Identity|hashing]] including the SCC construction, with property tests: α-invariance, determinism, round-trip.
- Ingest `set.mm` (~40k theorems) and re-export it byte-identically.

Success: re-ingesting `set.mm` a second time writes zero new nodes. Failure of this test means the canonicaliser is nondeterministic, which is the single most important thing to find out early.

**Measures:** ingest throughput (nodes/s), store size vs. source size, query latency for "fetch theorem with its proof tree".

## M1 — A real (tiny) language end to end

**Target: a Lisp** — s-expressions, `lambda`, `let`, integers, a handful of primitives. Now there are binders, so [[De Bruijn Index|de Bruijn]] conversion and [[Alpha Equivalence]] are exercised for real, and there is an actual [[Core Calculus|SC]] subset to elaborate into.

Deliverables:
- Surface AST → SC elaboration, SC kernel typechecking a simply-typed fragment.
- Functoriality property test on the frontend ([[Multi-AST Layering]]).
- `sophia-cli` can ingest, query and pretty-print back.

## M2 — Emit code

Lower the M1 Lisp subset through `func`/`arith`/`scf` [[MLIR Dialect|MLIR dialects]] to [[LLVM IR]] and run it. This is where "assembles IRs back into the graph database" becomes real rather than aspirational.

Deliverables: `LOWERS_TO` edges persisted; a working `sophia build`; the [[Compilation as Query|query-cost measurement]] that decides whether IR layers get persisted by default or treated as a cache.

**This milestone can kill the architecture.** If reconstructing IR from the store is an order of magnitude slower than rebuilding it from source, the design must retreat to "core terms persisted, IR regenerated", and that is worth knowing at M2 rather than M6.

## M3 — Julia ingestion and the precompilation win

Ingest Julia via `Meta.lower` and `code_typed`, record `DispatchFact` dependencies, and build the content-addressed code cache of [[Content-Addressed Precompilation]].

Deliverables: measurable time-to-first-execution improvement; measurable reduction in invalidation on method addition; cache sharing between two projects.

**This is the milestone that justifies the project to anyone who does not already believe in it**, and it does not require any of the equivalence machinery. It should be reached before anything cross-language is attempted.

## M4 — Equivalence within one language

Wire in `egg`, express a rule set over SC, saturate, extract, and store `EQUIV` edges with witnesses ([[Equivalence and Witnesses]]). Use them for the mundane and immediately useful case: [[Naming and Change Propagation|update propagation]] after an edit.

Deliverables: rewrite-chain witness format and its checker; the `modulo` discipline; the substitutability policy enforced at query time.

## M5 — Dependent types and the proof fragment

Add the `Prf` fragment: universes, inductive families, [[Normalization by Evaluation|NbE]]-based conversion. Attach specifications and proofs to Julia definitions ([[Tests and Documentation as Nodes]]). This is "Julia + Lean" in the only form that is actually tractable — a proof layer *about* Julia code, not dependent types *inside* Julia.

Deliverables: kernel that is small enough to audit; Lean-interop path (import Lean declarations as `Prop`/`Witness` nodes) rather than reimplementing Lean.

## M6 — Cross-language equivalence, first vertical slice

Pick one narrow, well-behaved domain — say, integer-only numerical kernels with no allocation — and demonstrate a witnessed Julia ↔ C++ equivalence, with the language-pair relation `R` written out explicitly and the [[Cross-Language Semantic Hazards|hazard]] list discharged. Not "C++ support"; one carefully chosen slice.

Deliverables: C++ ingestion via a Clang plugin, restricted to the slice; the `R` relation as a reviewed artifact; translation-validation witnesses on the lowering.

## M7 — Unified execution

Only now is the [[Start Here]] headline goal — "code from other programming languages just runs together, with no FFI" — even approachable, and only for code in the intersection of what all frontends can express. Expect the intersection to be small at first.

## What is explicitly *not* on the roadmap

- Writing a compiler backend. [[LLVM]] and [[MLIR]] do that.
- Reimplementing Lean. Import from it.
- A surface syntax for SC. Nobody writes core calculus by hand.
- Supporting all of C++. See [[Open Problems and Risks]].
- Distributed/remote stores before the single-machine case is fast.

## Ordering rationale

The riskiest assumptions, in order of how much they would cost to discover late:

| Assumption | Tested at | Cost if false |
| --- | --- | --- |
| Canonicalisation is deterministic and total | M0 | Everything |
| Store queries are fast enough to compile from | M2 | Retreat to IR-as-cache |
| Invalidation precision beats Julia's current scheme | M3 | The main practical justification evaporates |
| A rewrite rule set over SC is expressive enough to be useful | M4 | Equivalences stay `asserted` only |
| Cross-language `R` can be written and reviewed at all | M6 | Headline goal is unreachable; rest still stands |

Note the last row: even total failure at M6 leaves a content-addressed, queryable, precisely-invalidating code store, which is a worthwhile artifact on its own.

## Related

- [[Design Overview]]
- [[Open Problems and Risks]]
- [[Repository Layout]]
- [[Content-Addressed Precompilation]]
