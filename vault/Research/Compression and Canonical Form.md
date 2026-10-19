#comparison #design

> Daniel's question (SR-26): instead of only *normalising* statements, could the store *compress* them, so that the canonical form is no longer human-readable? Would that help higher-order languages, JIT compilation and macros? **Answer: three kinds of compression are worth having, and none of them should be the identity.** Structural sharing is already there. Byte compression is a storage detail. *Semantic* compression, which mines a corpus for abstractions, is the interesting one: it is what macros and higher-order functions do by hand, it finds reuse (Daniel's dream), and it can feed JIT specialisation and LLM vocabularies. It belongs in a **derived layer** whose results are proposals and facts, because a corpus-dependent compression would make ids change whenever unrelated code is added.
>
> Sources: Ellis et al., *DreamCoder*, PLDI 2021; Bowers et al., *Top-Down Synthesis for Library Learning* (Stitch), POPL 2023; Cao et al., *babble: Learning Better Abstractions with E-Graphs and Anti-Unification*, POPL 2023; Plotkin, *A Note on Inductive Generalization* (anti-unification), 1970; Larsson and Moffat, *Re-Pair*, 1999; Lohrey, *Grammar-Based Tree Compression*, DLT 2015; Turchin, *The Concept of a Supercompiler*, TOPLAS 1986; zstd dictionary training. code: none yet (planned `sophia-mine`).
>
> Theory (CT-ML wiki): [E-Graph](https://mathstruct.org/CategoryTheory-ML-Wiki/E-Graph) · [Polynomial Functor](https://mathstruct.org/CategoryTheory-ML-Wiki/Polynomial-Functor). This vault: [[Theory of the v0 Store]] · [[Block Size and Hash Overhead]].

## 1. The canonical form is already not human-readable

The stored form is the canonical bytes of the core term ([[SPEC-001 Sophia Store v0]] §4). It has no names, de Bruijn indices instead of variables, and references as 32-byte digests. Nobody reads it. People and models read **projections** ([[SPEC-005 Rendering Profiles]]).

So the question is not "readable or compressed". It is: **which compression, at which layer, and does it affect identity?**

## 2. Three kinds of compression

| Kind | What it does | Stable as identity? | Where it lives |
|---|---|---|---|
| **Structural sharing** (hash-consing, the Merkle DAG) | an identical definition or subterm is stored once and referenced by hash | **yes**: the result depends only on the term itself | already the identity (SPEC-001 §4) |
| **Byte compression** (zstd with a dictionary trained on the store, varint framing) | smaller pages on disk, more definitions per cache line | **no**: the dictionary depends on the corpus | the storage backend: transparent, measured ([[Benchmark Plan]]) |
| **Semantic compression** (abstraction mining: anti-unification, library learning, grammar compression of trees) | finds repeated *patterns with holes* across definitions, and proposes them as new abstractions that the originals can be rewritten to use | **no**: the best abstractions depend on the whole corpus, and adding code changes them | a derived layer: proposals, rewrite facts, a vocabulary |

Why semantic compression cannot be identity:
- **It depends on the corpus.** Minimum description length is relative to everything else stored. A definition's compressed form would change when an unrelated definition is added, so its id would change too. That breaks every cache, proof and test result keyed by it.
- **The optimum is uncomputable** (it is Kolmogorov complexity), and approximations depend on search heuristics, which would make ids depend on tool versions.
- **Normalising further is not compressing.** In higher-order languages, β-normal forms can be exponentially larger than the source. Inlining everything is the opposite of compression. The canonical form has to stay close to what was written, and only *irrelevant* differences (names, formatting) are normalised away.

## 3. What semantic compression buys

### 3.1 Reuse: finding the abstractions people did not write

Anti-unification of two terms computes their most specific common generalisation: `f(a, g(b))` and `f(c, g(d))` give `λx y. f(x, g(y))`. Library-learning systems do this across a whole corpus:
- **DreamCoder** grows a library while learning to solve tasks;
- **Stitch** searches top-down for the abstraction that compresses the corpus most, orders of magnitude faster;
- **babble** does it modulo equational theories, using e-graphs, so that abstractions are found even when the occurrences are only *equivalent*, not syntactically equal.

On a content-addressed store this becomes a query plus a search: *which abstraction, if introduced, would shorten the most definitions?* The result is a **proposal**:
- a new definition (the abstraction);
- rewrite facts "definition *d* is equal to the abstraction applied to these arguments", each with a checkable equivalence (β-reduction is the proof);
- a score in bits saved.

Accepting a proposal means:
- storing the abstraction;
- optionally re-expressing the originals through it;
- recording the equivalences as annotations.

Then tests need not re-run, because the equivalence is proved (SPEC-002 §8, case 3).

This is the most direct path to "seriously reuse code" (SR-73): the store can tell you that 40 definitions in three languages contain the same loop.

### 3.2 Macros

A macro is a hand-made compression: a pattern with holes, expanded at compile time. In a store:
- **A macro definition is a definition** (a term → term function, staged), and content-addressed.
- **A macro use is stored as written** (the call with its arguments). The **expansion is a derived fact** keyed by (macro id, argument ids), cached once per distinct use. De Bruijn indices prevent accidental capture of *bound variables* in the core, but they do **not** solve hygienic macros by themselves: phases, quotation, scope introduction and the meaning of free names in a template need their own specification (SR-86; corrected 2026-10-20).
- **Mining in reverse** finds code that *could* have been a macro use, and proposes the macro.

Storing uses rather than expansions keeps the stored program small and shows the reader what was written. Moonkale can draw both, the use and its expansion, as layers.

### 3.3 JIT compilation

A JIT specialises a method for concrete argument types and caches the machine code (SPEC-002 §7). Compression helps it in two places:
- **Sharing specialisations.** If two methods are instances of one mined abstraction, their specialisations for the same types are the abstraction's specialisation applied to constants. One compiled kernel can serve both, which means fewer compilations and less code in the instruction cache.
- **Deciding what to precompile** (SR-66). Frequency counts over abstractions, rather than over individual definitions, show which *patterns* are hot. That is a better precompilation unit than single methods.

The risk: a shared kernel may be slower than two individually specialised ones (constant folding is lost). It is a performance trade-off, decided by the benchmarks, never assumed.

### 3.4 Higher-order code

Higher-order code compresses better *as written* (a `map` with a lambda is shorter than its unrolled form) and worse *as compiled*, after inlining and specialisation. The store keeps the higher-order form as the identity. Compiled specialisations are derived. Supercompilation and partial evaluation are optimisations at L4, not identity transformations.

### 3.5 Models writing sophialang

Mined abstractions are a **vocabulary**: a BPE-like token set over trees instead of characters. A model trained on the store (SR-62) could emit abstractions directly. That shortens its output and makes reuse the default. It also means the vocabulary must be versioned and stored like any other derived fact.

## 4. Plan

| When | What |
|---|---|
| v0 (M1) | Structural sharing only, as the identity. Measure the dedup ratio on generated and real stores ([[Benchmark Plan]], B-07). |
| M2 | Byte compression in the backends: zstd with and without a trained dictionary, as a backend option. Measure size against read latency (B-08). |
| M6 (research) | `sophia-mine`: anti-unification over the store. First pairwise, then Stitch-style corpus search, then babble-style modulo e-graphs. Results are proposals with bit savings. Experiment: compress the sophialang standard library and the ingested Julia corpus, and report the abstractions found. |
| later | Macro expansion as a cached derived fact; JIT kernel sharing measured against per-method specialisation; a tree vocabulary for models. |

**The serious-benefit test** that Daniel asked for: if mining on real code finds abstractions that save more than ~10 % of the corpus, and people accept a meaningful fraction of them, the reuse benefit is real and the layer is worth building fully. If not, that negative result is worth recording too (SR-73).

## Related

[[SPEC-001 Sophia Store v0]] · [[SPEC-002 Sophialang Principles]] · [[Block Size and Hash Overhead]] · [[LLMs and Mixed-Language Code]] · [[State of the Art - Superoptimization and Synthesis]] · [[State of the Art - Equality Saturation and E-Graphs]]
