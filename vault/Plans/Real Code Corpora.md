#implementation #design

> Which real code B-16 uses, in what order, and why. Daniel, 2026-10-20: "Moonkale was meant as an editor for Lenticulum.jl. Now Sophia is somewhat intermingled with it, so it would be a good idea to think the three things together. Having Lenticulum run in Sophia would be a goal, but a later one; there should be easier Julia packages." He suggested BLAS-like linear algebra ("linear algebra might be quite fundamental"), Metamath, and then Lean for theorem libraries. Requirements: SR-114 to SR-118.
>
> Sources: `~/Code/CategoryTheory/Lenticulum.jl` (about 17 000 lines of Julia: `Lenticulum` plus five packages under `lib/`); Moonkale's WP-66 normaliser (`packages/services/index/src/content/`, tree-sitter grammars for Rust, Julia and Python via `arborium`); metamath/set.mm (GitHub). code: `crates/sophia-ingest` (Metamath first).

## 1. The three projects as one system

| | Role | What it needs from the others |
|---|---|---|
| **Lenticulum.jl** | the ML library: the program to be edited, stored and eventually run | nothing yet; later, to be **stored and run from Sophia** |
| **Moonkale** | the editor and graph view, originally built for Lenticulum | a code store with identity and queries (Sophia, SPEC-004) |
| **Sophia** | the store and, later, the language and compiler | real code to store: Julia first, because of Lenticulum |

Consequence for Sophia: **Julia is the first foreign language that matters.**
- Moonkale already computes WP-66 content ids for Julia, Rust and Python definitions with tree-sitter.
- Sophia's Julia ingestion should use **the same normal form**, ideally the same code (SR-116), so that a definition has one id in both.

## 2. The ladder

Each rung is a benchmark (B-16) and an experiment with a written result (SR-73).

| # | Corpus | Why | What it tests | Needs |
|---|---|---|---|---|
| 1 | **Metamath `set.mm`** (≈ 45 000 theorems; also `iset.mm`) | A real theorem library with a trivially parseable format and a huge, real dependency graph. Daniel named it | content addressing of a real library (renaming invariance, duplicate theorems); ingest at 10⁵-scale; deep transitive queries ("which axioms does `2p2e4` use?") | a Metamath frontend (`sophia-ingest`), no external tools |
| 2 | **Julia `LinearAlgebra` (generic parts)**, then **Octavian.jl** (BLAS written in Julia) | "Linear algebra is fundamental"; pure Julia; multiple dispatch everywhere | methods and generic functions (nominal entities, SP-006), dispatch-heavy reference graphs | a Julia frontend on the WP-66 normal form |
| 3 | **Rust `matrixmultiply`** (a pure-Rust gemm) | the same algorithm as rung 2 in another language | cross-language comparison: does Sophia find the same structure in Rust and Julia? (SR-73, the reuse dream) | a Rust frontend on WP-66 |
| 4 | **LuxCore.jl, then Lenticulum.jl** | the goal (SR-115) | a real project of the user's own, edited in Moonkale through Sophia | rung 2's frontend, plus macros and modules |
| 5 | **Lean 4 / Mathlib** (via `lean4export`) | a dependently typed theorem library | terms with real dependent types; Mathlib's scale | a Lean toolchain (not installed here) and an export reader |

Rung 1 does not contradict [[S-ADR-0008 Sophialang before Metamath or Lisp]]. That decision is about which **language Sophia implements** first. Here Metamath is **data to store and query**, through a small frontend.

## 3. Rung 1: Metamath into Sophia

Each assertion becomes one record of the language `metamath`:
- `$a` statements: syntax, axiom or definition;
- `$p` statements: theorems.

The design, as built after four corrections (§4):
- **A statement is the identity:** mandatory hypotheses in order, disjoint-variable pairs, typecode and math string. Variables are numbered by first occurrence; labels and variable names are display data. Its kind is `statement` or `syntax`. Statements are named `mm.<db>.<label>`.
- **A proof is evidence:** a separate record named `mmproof.<db>.<label>`. It references the **proofs** of the theorems it uses (the justification the author chose) and the **statements** of the axioms and definitions it uses. A `proof` annotation joins it to the statement it proves.
- **A library is a world** (SPEC-006 §1): three nominal collections, `mmlib.<db>.proofs`, `.axioms` and `.definitions`.

"Which axioms does `2p2e4` rest on, in set.mm" is then a plain walk:

```julia
query rests_on(proof::Id, axioms::Text)
    below(x) = refs(proof, x)
    below(y) = below(x) && def(x, :proof) && refs(x, y)
    select n where below(s) && def(s, :statement) && member(c, s) && name(axioms, c) && name(n, s)
end
```

## 4. Rung 1: results (2026-10-20)

**What the real library taught, in order.** Each point was found by running a real query, and each changed the design:

1. **Proofs are not identity.** The first normal form hashed each theorem's proof into its id. Then 12 059 of the 13 138 labels that set.mm and iset.mm share looked different, although the statements are equal: iset.mm proves them intuitionistically. Sophia's own principle (SPEC-002 §3) says a proof is evidence, not identity. With statements as identity, **12 927 of 13 138 are shared**, and the other 211 really differ.
2. **"Proves" is not "uses".** A proof referenced both the statement it proves and the statements it uses, as plain references. The query "the proof of s" then matched every proof that *uses* s: tens of thousands for `ax-mp`, and over the memory budget. Now a `proof` annotation carries "proves" (SPEC-001 §8). This is the review's point about typed relations (TypeQL's roles), met in real data.
3. **Being an axiom belongs to the library, not to the statement.** The record kind (axiom, theorem) was first-writer-wins, so set.mm theorems that state exactly what an axiom states were labelled axioms. Kinds now follow from content; roles are library collections.
4. **A step must name its justification.** With statements as identity, a proof step that referenced a statement lost *which* justification the author used. set.mm's personal sections prove `ax-1` from other axiom systems, and the walk followed those proofs. Now a step references the used theorem's **proof**, so a proof's identity covers its whole chain. Identical proofs between the libraries drop to 121, which is honest: their foundations differ.

**The answers.**
- In set.mm, 2 + 2 = 4 rests on 15 axioms:
  - logic: `ax-mp`, `ax-1` to `ax-9` and `ax-gen`, including classical `ax-3`;
  - set theory: `ax-ext`;
  - complex numbers: `ax-1cn`, `ax-addcl`, `ax-addass`.
  
  That is the list I would expect; it has not been cross-checked against `metamath.exe`.
- In iset.mm, the same statement rests on intuitionistic axioms (`ax-ia1`, `ax-io`, …) and **not on `ax-3`**.
- The fundamental theorem of algebra rests on 46 set.mm axioms.

**B-16 on set.mm** (51 159 assertions, 48 083 proofs, 98 136 records; median of 5):

| backend | assertions | proofs | records | parse | ingest (parse + store) | size | queries (median of 5) |
|---|---|---|---|---|---|---|---|
| sqlite | 51159 | 48083 | 98136 | 2.25s | 15.26s | 564 MiB | axioms behind 2p2e4: 15 rows, 3.87 ms; axioms behind fta (fundamental thm of algebra): 46 rows, 79.34 ms; proofs using ax-mp directly: 5361 rows, 14.53 ms; count by kind: 3 rows, 97.06 ms |
| redb | 51159 | 48083 | 98136 | 2.17s | 10.91s | 1028 MiB | axioms behind 2p2e4: 15 rows, 2.42 ms; axioms behind fta (fundamental thm of algebra): 46 rows, 46.53 ms; proofs using ax-mp directly: 5361 rows, 13.53 ms; count by kind: 3 rows, 66.98 ms |
| lmdb | 51159 | 48083 | 98136 | 2.17s | 6.21s | 791 MiB | axioms behind 2p2e4: 15 rows, 1.85 ms; axioms behind fta (fundamental thm of algebra): 46 rows, 37.14 ms; proofs using ax-mp directly: 5361 rows, 10.04 ms; count by kind: 3 rows, 52.63 ms |

**Two weaknesses found** ([[SP-011 Reference edges dominate storage on real code]]):
- **The store is large:** 564 MiB on SQLite for a 52 MB source. About 1.5 M reference edges (about 31 per proof, against 3 in the synthetic generator) take 42 % of the database, stored twice with 32-byte ids, plus a third copy on each record. H3 ("local ids save little") was refuted on synthetic code; on real code it is the main cost.
- **The planner guessed selectivity wrongly** twice: a lookup by kind, and by collection, is wide. It now knows that, and magic sets also apply to negated calls (SPEC-007 §6). Cost-based planning is the real fix.

**Next rungs:** a Julia frontend on Moonkale's WP-66 normal form (rung 2).

**Agreed with the Moonkale agent (2026-10-20, SR-116).** Moonkale extracts its normaliser into a small public crate, `moonkale-content-id` (`packages/services/content-id` in MathStruct/Moonkale):
- **contents:** the per-language normal forms, the SCC canonicalisation and hashing, the limits and `NORMALIZER_VERSION`;
- **dependencies:** only the arborium tree-sitter grammars and blake3;
- **output:** `[u8; 32]` digests, byte-identical to Moonkale's ids today;
- **input:** a parsed definition, plus a map from byte ranges to opaque target keys and their digests.

## 5. Rung 2: Julia LinearAlgebra (2026-10-20)

**Update, normaliser v4 (Moonkale `f09b3c40`).** Moonkale fixed both reported points:
- the self-edge is gone (v3);
- Rust struct expressions are uses (v3);
- every Julia local scope now has its own frame, and loops, varargs, destructuring and anonymous parameters are supported (v4).

Sophia's resolver matches Moonkale's on all golden fixtures, including the new `scopes.jl`, with no exceptions left. **LinearAlgebra: 3 947 of about 3 983 definitions get ids (99 %)**, with 36 refused (nested named functions, a few syntax errors). 894 generic functions with 3 484 methods, and 2 012 same-file references. The numbers below are from v2 and are kept as the record of why v4 was needed.

`sophia ingest-code julia LinearAlgebra <stdlib>/LinearAlgebra/src`, through the shared crate (`crates/sophia-ingest/src/code.rs`).

**Same ids as Moonkale.**
- On Moonkale's golden fixtures, with Moonkale's reference spans, every digest is identical in Julia, Rust and Python.
- With Sophia's own resolver, one difference remains: Rust `Point { … }` is a use for Sophia, not for Moonkale.
- Moonkale's Julia struct self-edge turned out to enter the id (the agent had thought not). Sophia mirrors it.
- Both points are reported to the Moonkale agent.

**Coverage of real Julia.** Of about 3 980 definitions in 34 files:
- **2 758 get an id** (69 %);
- **1 225 are refused** by the WP-66 v2 normal form, which conservatively rejects constructs whose scoping it does not model:
  - `for` loops (399) and anonymous functions (64);
  - about 729 for binding forms: varargs (`f(A...)`), tuple and named-tuple destructuring (`m, n = size(A)`), anonymous typed parameters (`::Type{T}`);
  - a few `do`, `let`, `catch` and comprehensions.
  
  Most numerical Julia uses these, so **the shared normal form is the bottleneck for Lenticulum** (SR-115). Reported to Moonkale with the categories.

**What is stored.**
- **Records:** 2 300 distinct ones. 458 definitions are exact duplicates of others: the same forwarding method written for several types.
- **Generic functions:** functions are grouped by name into 597 generic functions with 2 303 methods. `*` has 103 methods, `/` 71, `\` 54. This is SP-006's model, with the method table as a nominal relation.
- **Shared bodies:** 134 method bodies are members of two or more generic functions (for example `hpmv!` and `spmv!`). The function's own name is not part of the content, so identical bodies under different names are one record, the Unison idea.
- **References:** 913 same-file references. Cross-file references (one package spread over `include`d files) are not resolved yet, as in Moonkale.

**Module-level resolution (2026-10-20, [[SPEC-008 Module-Level Julia Resolution]]).**
- Per-file resolution is not only incomplete for Julia; it is partly wrong. About 1 940 per-file references in LinearAlgebra link a call to one method of a generic function, or a field `x.name` to a definition of that name.
- The module resolver makes 2 301 references, 880 of them across files.
- Proposed to Moonkale as normaliser v5.

**The caveat:** ids also depend on **reference resolution**. So "one id in both" needs the same resolver too. The plan has two steps:
1. Sophia uses the shared normaliser with its own resolver. Golden fixtures from Moonkale show where the two diverge, and the differences get recorded.
2. Moonkale moves its Julia resolver into the crate behind a plain-data interface, once Sophia ingests Julia.
