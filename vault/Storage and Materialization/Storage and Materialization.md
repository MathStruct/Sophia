#overview #design

> **What Sophia keeps permanently, and what it computes, uses and forgets.** Started 2026-10-20 at Daniel's request: "keep extensive documentation in the vault on what statements should actually be saved in the database and what intermediate statements can be cheaply generated. I don't mind if for a short time the memory usage goes high, but once the thing is computed not everything needs to be stored." Requirements: SR-119 to SR-121, SR-126 in [[Requirements]].
>
> Sources: [[SPEC-001 Sophia Store v0]] §8 (facts), [[SPEC-006 Worlds, Artifacts and Evidence]] §3, §6a; [[SP-011 Reference edges dominate storage on real code]]; [[Compression and Canonical Form]]; Nix and Bazel (input-addressed outputs, garbage collection from roots); Salsa and Adapton (demand-driven recomputation); materialized-view selection in databases (Harinarayan, Rajaraman and Ullman, *Implementing Data Cubes Efficiently*, SIGMOD 1996); Gupta and Mumick, *Maintenance of Materialized Views*, 1995.

## The rule in one sentence

**An object's identity, its provenance and the decision to keep its bytes are three separate things.** Sophia always keeps the first two for anything it reports. It keeps the bytes only when nothing cheaper can give them back.

Three questions are asked of every object, independently:

| Question | Answered by | Example |
|---|---|---|
| **What is it?** (identity) | its content id, or for a derived object its *input address*: H(operation, inputs, parameters, environment) | a Julia method's WP-66 id; the result id H(query, snapshot, params) |
| **Can it be rebuilt?** (reconstructibility) | whether the operation is deterministic, whether its inputs are still there, and what it costs | canonical bytes from source + normaliser version: yes, cheap; a proof search: yes, maybe hours; a measurement on hardware: no |
| **Should its bytes be kept?** (materialisation) | a policy: [[Materialization Policy]] | keep source text; drop canonical bytes once the id is known; cache a query result while it is hot |

Answering the third question "no" never changes the first two: a dropped object keeps its id, and a fact that cites it stays valid.

## Daniel's stance, as a requirement

- **Peaks are fine; residue is not.** Computation may use a lot of memory for a short time (a fixpoint, a compile, an AD tape), provided it is **released** when done (SR-120). The budget work in `sophia-query` already does this for queries: a byte estimate and a deadline bound the peak, and nothing of the working set survives the call ([[SPEC-007 Sophia Query Language]] §6.0).
- **Permanent storage is for what cannot be cheaply had again**: what people wrote, decisions made, evidence that was expensive to produce, and results that are both expensive and reused.

## The pages

| Page | What it says |
|---|---|
| [[Object Categories]] | the seven kinds of object, each with identity, provenance, reconstruction cost, reproducibility, invalidation, policy, GC and compression |
| [[Current Inventory]] | what Sophia **actually** stores today, table by table, and which of it is authoritative, redundant or derivable; measured on LinearAlgebra and set.mm |
| [[Object Lifecycle]] | compute → discard, cache or persist; promotion and demotion; what must survive when bytes are dropped |
| [[Materialization Policy]] | the default policy per category, the hard rules, and the cost model for later |
| [[Garbage Collection]] | roots, reachability, eviction, log compaction |
| [[Folder Store Garbage]] | how much of a replayed folder store is garbage, and a proposal to collect it |
| [[Compression]] | shrinking what *is* kept, independent of identity |
| [[AD as a Materialization Case]] | automatic differentiation: source, transformation, derived program, compiled artifact; when each deserves to be kept |
| [[Storage Measurements]] | what to measure, and the experiments that decide the open questions |

## The headline finding (2026-10-20)

Sophia today stores **about ten times the source it was given**, and most of it is derivable:
- Julia LinearAlgebra: 1.4 MB of source → 17 MB SQLite store, of which **12 MB are canonical bytes** that the shared normaliser rebuilds from the source text (the whole ingest, store writes included, takes 2.3 s);
- Metamath set.mm: 52 MB of source → 564 MiB, of which 42 % are reference edges kept three times.

Details and what to do about it: [[Current Inventory]]. Nothing is changed yet (SR-126: document before changing the storage layout).

## Invariants (hold for every policy)

1. **No authoritative object is dropped while it is reachable from a root** ([[Garbage Collection]]). Authoritative means: it cannot be rebuilt from anything else Sophia holds.
2. **Every derived object that is reported or cited has a recorded provenance**: the operation (name and version), the input ids, the parameters and the environment (world, toolchain, target) it depended on (SPEC-006 §3, §6a).
3. **A dropped derived object can be asked for again** by its provenance, and either is rebuilt with the same id, or the request fails visibly with "not reproducible" (non-deterministic, inputs gone, external). It is never silently replaced by something different under the same id.
4. **Identity never depends on materialisation.** Whether bytes are stored, compressed, deduplicated or recomputed does not enter any id (SPEC-006 §6: a physical-encoding change is not a migration).
5. **Transient working sets are bounded and released.** Peak memory is budgeted per operation; nothing stays resident after the operation returns, except what the policy explicitly caches.

## Related

[[SPEC-006 Worlds, Artifacts and Evidence]] · [[SP-011 Reference edges dominate storage on real code]] · [[SP-014 The query result cache is never evicted]] · [[Languages]] (which facts each language frontend produces) · [[Semantic Boundary Experiments]]
