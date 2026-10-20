#reference #open-problem

> The fourth external review of 2026-10-20: ChatGPT's review of the real-code work through `80f71ee` (Metamath, Julia on the shared normaliser v6), passed on by Daniel together with his own requests and his objection to LuxCore as the next step. It worked from the pushed code and vault, without compiling. This note records each point, whether it was right, and what was done. Requirements: SR-119 to SR-132 in [[Requirements]].
>
> Sources: the review text and Daniel's message (conversation of 2026-10-20); the code and measurements named below.

## Daniel's requests

| Request | What was done |
|---|---|
| Document **what is saved permanently and what is generated temporarily**; high temporary memory is fine, permanent storage should be minimal | New section [[Storage and Materialization]]: object categories, the current inventory (measured: the store is about 10× its source and mostly derivable), lifecycle, policy, garbage collection, compression, measurements, and AD as the worked case. Nothing in the layout changed yet (SR-126) |
| **A folder per language** documenting parsing and interoperability with sophialang | New section [[Languages]]: support levels L1–L6 and a matrix; Julia in eight pages; sophialang, the query language, Rust, Python, Metamath, Lisp and Lean in one each |
| **Not LuxCore next**: it avoids AD, Reactant and the other backends that full Lux brings | Agreed. [[Semantic Boundary Experiments]] replaces rung 4 of [[Real Code Corpora]]; Lux comes after the AD and Reactant experiments |
| AD as a test case for materialisation | [[AD as a Materialization Case]], with a first measurement (E-AD-0): producing a derivative costs 10² to 10⁶ runs of it |

## Verdicts on the review

| § | Point | Verdict | What was done |
|---|---|---|---|
| 1 | The M3a fixes were real | — | — |
| 2 | Statement and proof as separate identities; axiom as a library role | Agreed (it describes what was built) | — |
| 2 | **P0: the "2 + 2 = 4 rests on 15 axioms" answer is unchecked; check against metamath.exe** | **Correct, and done** | `sophia-mm-check` (new): metamath.exe built from source; Sophia and metamath.exe agree on **every theorem** of set.mm (48 083) and iset.mm (16 483), about 6.7 million dependencies compared. The 15 axioms of `2p2e4` are confirmed, plus 29 definitions |
| 2 | Storing a proof is not validating it | **Correct** | stated in [[Metamath Overview]]; metamath.exe checks all set.mm proofs in 2.85 s, so proof checking is the natural first trust record (E-0) |
| 3 | Surrogate keys for edges, hashes as public identity | Agreed (SP-011 option 1) | ordered in [[Compression]]: first drop the redundant refs blob, then surrogate keys |
| 4 | **P0: generic functions grouped by bare name merge `A.f` and `B.f`** | **Correct, and worse than stated** | Fixed: (module path, name) keys; `import` and qualified heads join the extended function ([[Julia Name Resolution]]). Measured against Julia's own method table: 53.7 % of LinearAlgebra's methods had the right owner before, 94.5 % after. Five real same-name merges (`dot`, `axpy!`, `axpby!`, `gemv!`, `check` across LinearAlgebra, BLAS, LAPACK), and 898 methods of Base functions filed as LinearAlgebra's own. Test `tests/generics.rs` covers the cases the review listed (unrelated same-named generics, imports, qualified extensions, constructors) |
| 5 | Distinguish generic identity, method table at a world, and dispatch result; in SPEC-006 | **Correct** | SPEC-006 §1a; [[Julia Multiple Dispatch and World Age]] |
| 5 | Identifier spelling will not be enough for macros, generated functions, runtime methods | **Correct** | [[Julia Macros and Generated Functions]]: Julia as the oracle; expansions as transformation records (E-4) |
| 6 | A Julia semantic conformance suite before more packages | **Agreed** | [[Julia Conformance]]: parts 1 (fixture) and 2 (LinearAlgebra vs Julia) done; parts 3–9 planned. It found a bug in the shared crate (units inside macro arguments), which Moonkale fixed the same day in v7 (now 95.1 %) and an approximation (re-exported bindings, [[SP-013 Julia bindings of modules Sophia has not ingested]]) |
| 7 | P1: enforce the record-kind invariant (SP-012) | **Agreed, done** | `StoreError::KindConflict`; conformance test on four backends |
| 7 | P1: Moonkale edit-latency benchmark | Agreed | still waits for Moonkale on the current revision (B-15) |
| 7 | P2: compact reference keys; a representative part of Lenticulum | Agreed on order | [[Compression]]; Lenticulum after E-8 |
| 7 | Keep SQLite default until measured | Agreed | unchanged (S-ADR-0010) |

## What the review got wrong or missed

- **The generic-function issue was larger than predicted.** The review's example was two sibling modules with the same function name (5 cases in LinearAlgebra). The bigger error was methods of **other modules'** functions (Base's `*`, `show`, `getindex`, …) stored as the package's own: 898 methods. Only running against Julia's method table showed the size.
- **Not everything a static tool gets wrong is a bug in it.** 86 of the remaining disagreements come from Base's re-exports (`Base.Matrix` is `Core.Array`), which no reading of LinearAlgebra's source can settle. The fix is a binding table produced by Julia, which is the review's own point (§5) applied to names.
- **The materialisation question has a concrete answer already.** The review proposed a cost model; the measurement shows the first decision needs none: for ingested code the canonical bytes (70 % of the LinearAlgebra store) are derivable from the source in seconds.
- **AD numbers.** The review expected compiled derivatives to be worth caching mainly for large models. E-AD-0 shows that even toy functions take 0.1–5 s to differentiate and compile, paid again in every Julia session: persisting compiled derivatives pays early.
