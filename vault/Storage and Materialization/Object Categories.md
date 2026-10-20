#design #definition

> The kinds of object Sophia handles, and for each: identity and provenance, cost of reconstruction, reproducibility, dependencies and invalidation, materialisation and eviction, garbage collection, compression, and the benchmark that measures it. Part of [[Storage and Materialization]]. 2026-10-20.
>
> Sources: [[SPEC-001 Sophia Store v0]] §4–§8; [[SPEC-006 Worlds, Artifacts and Evidence]]; [[Current Inventory]]; the external review of 2026-10-20 ([[Review 2026-10-20 Real Code (external)]]), whose four categories this refines into seven.

## Overview

| # | Category | Examples in Sophia | Authoritative? | Default policy |
|---|---|---|---|---|
| 1 | **Authored content** | sophialang definitions written in Moonkale; source text of ingested files; Metamath `.mm` text | **yes**: nobody can recompute what a person wrote | persist, forever while reachable |
| 2 | **Canonical forms** | a definition's canonical bytes; its parts with resolved references; its content id | no: a function of (1) and a normaliser version | keep the **id** and the **references**; the bytes are a cache |
| 3 | **Decisions and mutable state** | names, lineage, nominal collections (method tables, library roles), world snapshots, the change log | **yes**: a choice, not a computation | persist; compact the log ([[Garbage Collection]]) |
| 4 | **Evidence** | proofs, certificates, test results, transformation records (SPEC-006 §6a), trust records | sometimes: a proof found by search is expensive; a test run on hardware is not reproducible | persist selectively, always with provenance |
| 5 | **Derived structures** | reverse edges, kind indexes, dependency closures, query results, inferred relations, dispatch resolutions | no | recompute, index or cache by cost |
| 6 | **Compiled artifacts** | object code, pkgimages, compiled derivatives, MLIR modules | no, but rebuilding may be very expensive and environment-dependent | cache by artifact id A (SPEC-006 §3); persist the expensive and reused |
| 7 | **Runtime intermediates** | parse trees, query working sets, join tables, AD tapes, compiler pass outputs, traced graphs | no | never persisted unless promoted ([[Object Lifecycle]]) |

The line between 2 and 5 is that a canonical form is *the* representation an id is computed from. A derived structure is anything else computed from the store.

## 1. Authored content

- **Identity:** content id of the canonical form (2). For an ingested file, also the **source blob**, content-addressed by its bytes (proposed; Sophia does not keep source files today).
- **Provenance:** who wrote it, when, through which tool (the change log, SPEC-001 §7).
- **Reconstruction:** impossible. This is the one category that must never be lost.
- **Reproducible:** not applicable.
- **Invalidation:** none; it is immutable. Edits make new content.
- **Policy:** persist. For ingested code, **the source text is the authority**, not the canonical form: the canonical form is a function of it.
- **GC:** kept while reachable from a name, a world, a lineage or a pinned artifact.
- **Compression:** general-purpose (zstd) works well on text; dictionary compression across similar files. Deduplicate by content.
- **Benchmark:** bytes per definition; source vs store ratio ([[Storage Measurements]] M-1).

## 2. Canonical forms

- **Identity:** they *are* the identity's preimage: id = H(canonical bytes).
- **Provenance:** the normaliser and its version (`moonkale-wp66@6`, `sophia-metamath@2`, `sophialang-core@0`), recorded on every record.
- **Reconstruction:** from (1) plus the normaliser and the resolved references. Cost: linear in source size. LinearAlgebra's whole module: 2.3 s including storage.
- **Reproducible:** yes, by construction: that is what a normal form is for. Guarded by golden tests (Moonkale's fixtures) and by re-hashing on verification.
- **Dependencies:** the normaliser version. A normaliser change is a **migration** (SPEC-006 §6): new ids, and `migrated_to` relations.
- **Policy, proposed:**
  - keep the **id**, the **kind**, and the **reference edges** (the graph that queries walk);
  - keep the **canonical bytes** only where no authored source exists: sophialang definitions created in Sophia, whose canonical form *is* the authored form (here 1 and 2 coincide);
  - for ingested foreign code, treat the bytes as a cache: rebuild on demand for verification, printing or diffing.
- **GC:** with their authored source.
- **Compression:** the WP-66 part encoding is verbose (≈ 9× the source for LinearAlgebra). If kept, compress, or store parts as references into the source plus a small delta.
- **Benchmark:** M-2 (rebuild cost per definition vs storage saved).

## 3. Decisions and mutable state

- **Identity:** names by path; nominal entities by fresh id; worlds by content (SPEC-006 §2); log entries by sequence number.
- **Provenance:** the log.
- **Reconstruction:** the current state can be rebuilt by replaying the log; the log itself is authoritative.
- **Reproducible:** replay is deterministic.
- **Invalidation:** a change makes new world snapshots possible; facts that cited an old world stay valid *for that world*.
- **Policy:** persist the current state and the log. **Worlds** are persisted when cited (by a fact, an artifact, a test run), otherwise they are just a function of the log at a sequence number and need not be stored.
- **GC:** log compaction to snapshots after a retention period; uncited worlds are not kept.
- **Compression:** worlds that share structure (a Merkle tree per nominal entity), when B-09 says so.
- **Benchmark:** B-09 (snapshot cost), M-5 (log growth under Moonkale editing).

## 4. Evidence

- **Identity:** content id of the evidence (proof term, certificate, test log); the claim it supports is separate (SPEC-006 §3: C, P and A are never conflated).
- **Provenance:** checker or tool, its version and trusted kernel, inputs, world (SPEC-006 §5).
- **Reconstruction:** varies enormously:
  - a Metamath proof: authored, so category 1, and checking it is cheap;
  - a proof found by an SMT solver or a tactic search: rebuildable in principle, but at search cost, and the search may not find the same proof;
  - a **certificate** (LRAT, a proof term): cheap to *check*, expensive to *find*: the classic case for persisting;
  - a test result: a measurement. Re-running gives a new measurement, not the same object.
- **Reproducible:** deterministic checkers yes; tests and timing measurements no.
- **Invalidation:** the claim goes **stale** when its inputs change (SPEC-001 §4.5); the evidence itself stays true about the inputs it names.
- **Policy:** persist **certificates** and anything whose finding cost greatly exceeds its checking cost; persist test results that a build or a decision cites; drop the rest after a retention window. Keep the provenance record even when the evidence bytes are dropped, so "this was once checked" stays answerable.
- **GC:** roots are the artifacts and decisions that cite the evidence.
- **Compression:** proof terms share subterms heavily (hash-consing); Metamath's compressed proof format is the existence proof.
- **Benchmark:** M-4 (finding vs checking cost).

## 5. Derived structures

- **Identity:** **input-addressed**: H(operation id, input ids, parameters, world). The query result id H(Q, S, p) is the model.
- **Provenance:** that same tuple, stored with the result.
- **Reconstruction:** run the operation again. Cost from microseconds (a reverse edge) to minutes (a full closure).
- **Reproducible:** yes, if the operation is deterministic and its inputs are in the store.
- **Invalidation:** by input change. Snapshot-keyed results never go wrong: a newer snapshot just has a different id. Stale entries are garbage, not errors.
- **Policy:** three tiers:
  - **indexes** (reverse edges, kind and member indexes): persisted as part of a backend's physical layout, because every query needs them; counted as storage cost, never as identity;
  - **caches** (query results, closures): kept while hot, evicted by cost ([[Materialization Policy]]);
  - **on-demand** (everything else): computed per request, released after.
- **GC:** evict by cost and age; never a root.
- **Compression:** results are small; indexes are where compression matters ([[SP-011 Reference edges dominate storage on real code]]).
- **Benchmark:** B-17 (query times with and without cache), M-3.

## 6. Compiled artifacts

- **Identity:** A = H(C, world, compiler, version, target, flags, libraries, evidence used) (SPEC-006 §3).
- **Provenance:** the same tuple, plus the build log.
- **Reconstruction:** rebuild. Cost from milliseconds (interpreting) to hours (whole-program optimisation, Reactant compiling a large model through XLA).
- **Reproducible:** only if the toolchain is deterministic; many are not bit-for-bit (timestamps, parallel codegen, autotuning). A non-reproducible artifact must record that, and two builds of it are two artifacts.
- **Invalidation:** none, because A covers its inputs; a changed input means a different A.
- **Policy:** cache by A; persist those that are expensive **and** reused (a compiled derivative of a model trained many times); pin the artifacts a release or a binary ships.
- **GC:** pins and releases are roots; the rest is a cache.
- **Compression:** standard (zstd); deduplicate identical outputs by content hash as well as by A.
- **Benchmark:** M-6, with AD as the first case ([[AD as a Materialization Case]]).

## 7. Runtime intermediates

- **Identity:** none by default. An intermediate gets an identity only when it is **promoted** ([[Object Lifecycle]]).
- **Provenance:** not recorded unless promoted.
- **Reconstruction:** they exist only during a computation.
- **Policy:** never persisted by default; bounded by the operation's budget; released at the end (SR-120).
- **Examples:** tree-sitter parse trees, the semi-naive executor's delta relations and indexes, SQL temporary tables, AD tapes (Zygote's pullback closures, ReverseDiff's tape), MLIR modules between passes, Reactant's traced graph before compilation.
- **Promotion examples:** an MLIR module worth inspecting in Moonkale (a lowering explorer); an AD tape reused for many gradient calls on the same input structure; a query plan reused across parameters.

## Related

[[Current Inventory]] · [[Materialization Policy]] · [[Object Lifecycle]] · [[Garbage Collection]] · [[Compression]]
