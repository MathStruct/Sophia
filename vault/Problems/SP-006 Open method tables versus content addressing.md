#open-problem

> **Status: open, with a proposal (2026-10-20).** Related: [[SPEC-002 Sophialang Principles]] §4, [[SPEC-001 Sophia Store v0]] §4.3, [[Operating Systems and Sophia]] §4, [[World Age]], [[Multiple Dispatch]].

**Problem.** Multiple dispatch (SR-59) means a generic function's methods form an **open** set: anyone can add a method to `+` later. Content addressing (SR-01) wants every definition's identity to cover its dependencies.

If a caller of `+` referred to "`+` with its current methods" by hash, then adding one method to `+` would change the hash of every function in the store that adds numbers, and of every function that calls those. That would be an invalidation storm, the persistent form of Julia's invalidation problem.

**Proposal.**
- Generic functions are **nominal** entities (a fresh id). Callers reference the id, so adding a method changes no caller's content id.
- The method table is a **versioned relation**.
- Dispatch resolutions, compiled specialisations and test results are **derived facts** that record the method-table versions they relied on. A new method makes stale only the facts whose recorded call signatures overlap its signature. That is Julia's backedges and world age, made persistent and queryable.

**What remains hard.**
- The *meaning* of a caller now depends on mutable state (the method table), so two stores with the same caller id but different method tables can run it differently. Builds and test results must therefore pin method-table versions, and they do (Minimal Binaries §3).
- **Per-environment method tables** (CLOSOS's multiple global environments, Operating Systems §4): a user's extra method should not affect others. The relation would then be versioned per environment.
- **Method ambiguity** (two equally specific methods) must be detected when a method is added, not at call time. That needs the subtyping algorithm, which is Julia's hardest corner.

**World snapshots** (SPEC-006 §2, implemented 2026-10-20) give every result a fixed world to be reproduced in. Minimal invalidation by signature overlap is still open (SR-85).

**Measured (2026-11-19, E-3, [[Dispatch Invalidation]]):** on a fixture and on LinearAlgebra, with dispatch facts recorded from Julia, the overlap rule never missed a changed call or an instance Julia invalidated. Overlap alone is coarse for a method less specific than the existing ones (`abs(x) = x`: 16 false calls, 135 instances); with a specificity check (a method the call already reaches covers it and beats the new one) it was exact on calls. For compiled code Julia invalidates less than any rule over dispatch facts can: a call on `Any` whose reachable methods change has no backedge if inference never used its result (LinearAlgebra: 0 invalidations, 13 instances relying on changed calls). Ambiguity detection was not measured.

**Why it might fail** (SR-73): if the overlap check is too coarse, every new method on a popular generic function still invalidates most facts. Measure it with B-09 (invalidation precision) once a Julia corpus is ingested.
