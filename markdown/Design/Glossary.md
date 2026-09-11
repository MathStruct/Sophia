# Glossary

Project vocabulary, with the ambiguous terms disambiguated. Where a word means something different inside Sophia than in general usage, that is called out.

**Annotation** — in [[Start Here]]'s sense: any node in the graph that is not compiled but says something about code. Covers docs, tests, specifications and equivalence claims. See [[Tests and Documentation as Nodes]].

**Canonical form** — the normalised shape of a term that hashing operates on. Produced by the canonicaliser; its definition is a permanent commitment because changing it changes every identity. [[Hashing and Identity]]

**Cmp** — the general-recursive, effectful fragment of [[Core Calculus|SC]] that all programs live in. Logically inconsistent by design; contrast `Prf`.

**Content addressing** — identifying data by a hash of its own content. [[Content-Addressed Code]]

**Decl** — a named top-level definition node: term + type + effect row. [[Graph Schema]]

**Definitional equality** (`≡`) — equality decided silently by the kernel via normalisation. Folded into the hash. Contrast propositional equality. [[Definitional vs Propositional Equality]]

**Elaboration** — translating a surface language into [[Core Calculus|SC]], inserting all the information the surface syntax left implicit. What a frontend does.

**e-graph / equality saturation** — data structure and technique for storing many equivalent expressions compactly and deriving new equivalences by rewriting. [[E-Graph]], [[Equality Saturation]]

**EQUIV edge** — a stored claim that two terms are equivalent, carrying a `level` and a `modulo` set. [[Equivalence and Witnesses]]

**Extraction** — choosing one representative term per equivalence class, minimising a cost model. [[Compilation as Query]]

**Frontend** — a per-language extractor producing surface nodes and their elaborations. Untrusted. [[Trusted Computing Base]]

**h_def / h_type / h_run** — the three hashes of a definition: full identity, interface, and erased executable form. [[Hashing and Identity]]

**Layer** — position on the abstraction axis: surface → core → MLIR → LLVM. Distinct from the term/type/proof axis. [[Multi-AST Layering]]

**Level** (of an `EQUIV` edge) — strength of an equivalence claim, from `alpha` down to `asserted`. Only the top four license substitution.

**Modulo** — the set of things an equivalence claim ignores (`alloc`, `fp_assoc`, `timing`, …). Unions under composition, which is the main way chained claims become useless.

**Name** — a mutable `(namespace, symbol) → hash` binding. The only mutable relation in the store. [[Naming and Change Propagation]]

**Prf** — the total, effect-free fragment of [[Core Calculus|SC]] in which witnesses and specifications live. Logically consistent.

**Propositional equality** (`=_A`) — equality as a proposition requiring a proof term. What an `EQUIV` witness carries.

**Refinement** (`⊑`) — the asymmetric sibling of equivalence: `a ⊑ b` means `a` may be substituted for `b`, not conversely. [[Equivalence and Witnesses]]

**Region** — an abstract memory area in the effect/ownership system; the common device unifying GC heaps, arenas and Rust lifetimes. [[Effects Memory and Resources]]

**SC (Sophia Core)** — the common core calculus every frontend targets. [[Core Calculus]]

**SCC hashing** — the construction that makes Merkle hashing well-defined on mutually recursive definitions. [[Cycle Hashing]]

**Surface node** — a node of a source language's own AST, before elaboration. Carries spans and names; never part of core identity.

**Target** — a node identifying triple, datalayout and CPU features. Lowering and benchmarks are only meaningful relative to one.

**TCB** — trusted computing base: the components that must be correct for any claim in the store to mean anything. Notably *excludes* the database itself. [[Trusted Computing Base]]

**Translation validation** — checking, per compilation instance, that output is equivalent to input; as opposed to proving a compiler correct once and for all. [[State of the Art - Program Equivalence Checking]]

**Witness** — a node carrying evidence for a claim: a proof term, a rewrite chain, an SMT certificate, a validation report, test results, or a signature.

**World age** — Julia's notion of a point in the evolution of the global method table; every equivalence claim about a generic Julia function is implicitly relative to one. [[World Age]]

## Terms deliberately avoided

- **"UUID"** — [[Start Here]] uses it for the content hash. This vault says *hash* to avoid confusion with RFC 4122 UUIDs, which are 128-bit and mostly random. A UUID-shaped identifier can be derived by truncation, for display only.
- **"AST"** unqualified — always say which layer and which axis; see [[Multi-AST Layering]].
- **"Proof"** for test evidence — tests are evidence, not proof, and the distinction is load-bearing.
- **"Equivalent"** without a modulo — see above.

## Related

- [[Design Overview]]
- [[Wiki Home]]
