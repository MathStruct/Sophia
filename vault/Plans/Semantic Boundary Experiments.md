#plan #experiment

> **The revised experimental roadmap (2026-10-20).** It replaces the "bigger packages next" order of [[Real Code Corpora]] rung 4 (LuxCore, then Lenticulum). Daniel: LuxCore "proves relatively little, as it avoids the AD and Reactant or other backend libraries that full Lux.jl brings with it." The external review: "demonstrate that the stored graph accurately represents what the code actually means." Each experiment has a hypothesis, a method, measurements and a failure criterion. Requirements: SR-124, SR-125, SR-126, SR-129 in [[Requirements]].
>
> Sources: [[Review 2026-10-20 Real Code (external)]]; [[Languages]]; [[Storage and Materialization]]; [[AD as a Materialization Case]]; [[Julia Conformance]].

## Principles

1. **Semantic boundaries before scale.** The question is not "how many definitions get ids" but "which facts does Sophia infer correctly, which intermediate objects are worth keeping, which relations hold between representations".
2. **An oracle for every claim.** Each experiment compares Sophia with an independent implementation: metamath.exe, Julia itself, rustc or rust-analyzer, numerical agreement between AD backends.
3. **Specifications, small experiments, benchmarks.** No new major subsystem (SR-126). Where an experiment needs a capability (a Julia process in the loop, IR layers), it states it, and the decision to build it is separate.
4. **Moonkale stays a priority** (SR-125): B-15, the edit-latency benchmark, runs in parallel as soon as Moonkale is on the current revision.

## Done (2026-10-20)

| # | Experiment | Result |
|---|---|---|
| E-0a | Metamath dependency analysis vs metamath.exe | agrees on every theorem checked (3 000 set.mm, 1 001 iset.mm sampled; full run: [[Real Code Corpora]] §4) |
| E-1a | Julia generic ownership vs Julia's method table | fixture: all cases; LinearAlgebra: 94.5 % (was 53.7 % before the (module, name) fix) |
| E-AD-0 | cost of producing vs running D(f), five AD systems | producing costs 10² to 10⁶ runs; 0.1–6.6 s per function and signature, paid again every Julia session |

## The experiments, in order

### E-0 Evidence with trust: Metamath proof checking

- **Hypothesis:** SPEC-006 §5 trust records work end to end on a real library: each proof gets `checked by <checker, version>`, and a query can ask "which theorems rest only on checked proofs".
- **Method:** run metamath.exe `verify proof *` (2.85 s for set.mm), record per-proof trust records; optionally a minimal Rust verifier as a second checker.
- **Measures:** time; size of trust records; query time for the trusted closure.
- **Fails if:** trust records cost more than the proofs they annotate, or the trusted-closure query is not expressible in the query language.
- **Teaches:** evidence vs storage ([[Object Categories]] 4), the first real use of trust levels.

### E-1 Julia conformance, parts 3–9

- **Hypothesis:** for each construct (dispatch, world change, macros, closures, mutable globals, parser agreement, include order) Sophia's facts fall into knows / approximates / leaves open, with **no "wrong"** after fixes.
- **Method:** one fixture package per construct plus a Julia script printing the ground truth ([[Julia Conformance]]).
- **Measures:** counts per outcome, per construct, on the fixture and on LinearAlgebra.
- **Fails if:** a "wrong" cannot be fixed or isolated (flagged as unknown).
- **Teaches:** the ledger of what Sophia may claim about Julia.

### E-2 Cross-language structure: Rust `matrixmultiply` vs Julia matmul

- **Hypothesis:** the reference graphs of two independent gemm implementations share recognisable structure (packing, blocking, micro-kernel) that a query can find without names.
- **Method:** ingest `matrixmultiply` (Rust) and Julia's generic matmul (LinearAlgebra `matmul.jl`) or Octavian.jl; compare graph shapes (depth, fan-out, loop nests) by query.
- **Measures:** ingest coverage; graph statistics; whether a structural query matches corresponding parts.
- **Fails if:** nothing beyond trivial shape is shared; that would say structural reuse needs a semantic IR first (L4), which is itself a result.
- **Teaches:** whether cross-language reuse (SR-73) is reachable at L3 or needs L4.

### E-3 Dispatch facts from Julia

- **Hypothesis:** dispatch results recorded from Julia (`which(f, Tuple{…})`, citing a world) are enough to invalidate exactly the affected facts when a method is added.
- **Method:** a fixture with a method table and call signatures; add a method; compare Sophia's stale set (signature overlap) with Julia's invalidations (SnoopCompile's `@snoop_invalidations`).
- **Measures:** precision and recall of the stale set.
- **Fails if:** signature overlap is far coarser than Julia's backedges (many false stales).
- **Teaches:** SP-006 and SR-85 on real dispatch; [[Julia Multiple Dispatch and World Age]].

### E-4 Macro expansions as transformation records

- **Hypothesis:** recording `macroexpand` output as a transformation record (input address: macro id, call id, world) makes macro-generated methods visible, closing most of the 1 026 methods Sophia cannot see in LinearAlgebra.
- **Method:** expand LinearAlgebra's module-level macro calls and `@eval` loops in Julia; ingest the expansions as derived programs.
- **Measures:** methods recovered; cost of expansion; size of the expansions.
- **Fails if:** expansions are not reproducible from the recorded inputs (hidden state).
- **Teaches:** derived programs with provenance, on the cheapest possible transformation.

### E-5 Rules vs derived derivatives (ChainRules, ForwardDiff, Zygote)

- **Hypothesis:** id(D(f)) can be computed **before** running D, from (D, version, mode, f's closure, rule world, argument types), and two runs with equal keys give identical derived programs.
- **Method:** small Julia functions using rules and plain code; obtain Zygote's adjoint IR and record its inputs; add an `rrule` and observe which D(g) change.
- **Measures:** whether equal keys give equal outputs; which keys change when a rule is added; transformation vs compile time (separating the two costs E-AD-0 mixed).
- **Fails if:** hidden inputs make D(f) differ under equal keys.
- **Teaches:** input addressing of derived programs; world dependence of transformations ([[AD as a Materialization Case]]).

### E-6 One derivative, two layers (Zygote vs Enzyme)

- **Hypothesis:** Sophia can store a derived program at Julia IR (Zygote, Mooncake) and at LLVM IR (Enzyme) and relate them by a `tested`-level equivalence claim on the function they compute.
- **Method:** export both derived programs for a few functions; store as records of two IR languages; record numerical agreement as evidence.
- **Measures:** sizes; agreement; what the store needs to hold LLVM IR.
- **Fails if:** the layers cannot be stored without a new subsystem; then it is a specification result, not code.
- **Teaches:** [[Multi-AST Layering]] on real data.

### E-7 Staged compilation (Reactant)

- **Hypothesis:** a Reactant/XLA executable is the clearest case for persisting a compiled artifact: compile time dominates, the artifact is deterministic per (shapes, types, flags, versions), and it is reused across steps and sessions.
- **Method:** compile a small Lux model's training step with Reactant; measure compile time, artifact size, determinism across runs.
- **Measures:** compile time vs step time; artifact bytes; bitwise reproducibility.
- **Fails if:** artifacts are not reproducible (then A must include a nonce, and persisting is the only way to have them again).
- **Prerequisite:** `Reactant_jll` must be installed (it did not resolve offline here; it is a large download: Daniel's call).

### E-8 Full Lux.jl

After E-5 to E-7: ingest Lux with its AD and Reactant integrations; the test is whether the facts E-1 to E-7 established hold on a real, interlocking package. Not before.

### E-9 Lenticulum.jl

Later (SR-115). The real application, once the boundaries are understood.

## Parallel tracks

| Track | Item | Why parallel |
|---|---|---|
| Moonkale | **B-15 edit latency** (SR-125) | the product path; needs Moonkale on the current revision |
| Storage | M-1, M-2 ([[Storage Measurements]]): a smaller layout, rebuild costs | independent of Julia semantics; needs Daniel's agreement before changing the layout |
| Store | SP-012 enforcement (record kind follows content) | correctness, small |
