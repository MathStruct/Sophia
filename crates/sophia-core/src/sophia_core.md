# sophia_core

The Sophia Core calculus, its kernel and its representation. Source: `sophia_core.rs` (comments only).

Full design: [[Core Calculus]].

## What this crate owns

The one language every frontend elaborates into, and the kernel that checks it. Everything else in the workspace is downstream: [[sophia_hash]] hashes core terms, [[sophia_store]] stores them, [[sophia_equiv]] relates them, [[sophia_emit]] lowers them.

## Three design decisions worth restating

**Locally nameless representation.** `Var(DbIndex)` for bound variables, `Ref(Hash)` for free ones. Free variables refer to other definitions *by identity*, not by name — which is what makes [[Alpha Equivalence|α-invariance]] and [[Content-Addressed Code|content addressing]] structural rather than a post-hoc check. See [[De Bruijn Index]].

**Primitives carry their semantics.** There is no abstract `Int` in SC. `IntArith` carries width, signedness and an `Overflow` discipline (`Wrap` for Julia, `Trap` for Rust-debug, `Poison` for C++ `nsw`); `FloatArith` carries format, rounding, and contraction/reassociation permission. This is verbose and it is the point: Julia's `+` and C++'s signed `+` are different functions, and a calculus that cannot say so will happily support a false equivalence claim. See [[Cross-Language Semantic Hazards]].

**Two fragments.** `Cmp` has general recursion and effects, is where all programs live, and is logically inconsistent — `Fix` inhabits every type. `Prf` is total and consistent and is where [[Equivalence and Witnesses|witnesses]] live. There is an inclusion `Prf ↪ Cmp` and deliberately no map back; statements *about* `Cmp` terms are made in `Prf` by quoting them by hash. Without this split every proof in the store would be vacuous, so the kernel enforces it rather than trusting convention.

## The invariant that ties this crate to sophia-hash

`convert` (definitional equality, decided by [[Normalization by Evaluation|NbE]]) and the normalisation step inside [[canonical]] **must be the same function**. If they diverge, a term can enter the store under two identities and content addressing stops meaning anything. This is the least obvious coupling in the workspace and is worth a test that exercises both paths on the same corpus.

## Submodules

- [[elaborate]] — surface → core, NbE, unification, implicit arguments
- [[effects]] — effect rows, regions, borrows

## Related

- [[Core Calculus]] · [[Dependent Types]] · [[Definitional vs Propositional Equality]]
- [[Hash Consing]] · [[Hashing and Identity]]
- [[Trusted Computing Base]] · [[Repository Layout]]
