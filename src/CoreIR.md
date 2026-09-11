# CoreIR

Julia-side mirror of the Sophia Core term language. Source: `CoreIR.jl` (comments only).

Full design: [[Core Calculus]]. Rust counterpart: [[sophia_core]].

## Why duplicate a type that already exists in Rust

Ergonomics is the obvious reason — building and inspecting core terms in the REPL is how a frontend gets debugged. The real reason is **differential testing**.

[[canonical]] and the hashing path are in the [[Trusted Computing Base]], and their failure mode is silent. A second, independent implementation that agrees on every digest across a large corpus is meaningful evidence that what is implemented is the *specification* rather than one program's idiosyncrasies. It is the cheapest such check available, and it is available precisely because this repository already spans two languages.

The cost is a definition that can drift. Mitigation: generate both sides from one schema file, or at minimum run the conformance suite in [[Hashing]] on every change.

## Primitives keep their attributes here too

Julia's `+` on `Int64` is `IntArith(:add, 64, true, Wrap)`. C++'s signed `+` is `IntArith(:add, 64, true, Poison)`. Different nodes, different hashes, deliberately — see [[Cross-Language Semantic Hazards]]. The point of making the Julia mirror carry the same attributes is that a frontend written in Julia cannot quietly emit an under-specified primitive because the local type made it easy.

## Fragments enforced at construction

`Fix` is legal only in `Cmp`. The constructor should *refuse* to build a `Prf` term containing it rather than deferring to a later check — an inconsistent proof fragment makes every witness in the store vacuous, and that is not a failure anyone will notice downstream. See [[Core Calculus]].

## Printing

`show` should resolve `Ref` hashes through the name table and print readable syntax; [[De Bruijn Index|de Bruijn]] indices are correct and unreadable. A raw mode matters for the case where the names themselves are what is under suspicion.

## Related

- [[Core Calculus]] · [[sophia_core]] · [[Hashing]]
- [[De Bruijn Index]] · [[Linear and Affine Types]]
- [[Cross-Language Semantic Hazards]] · [[Trusted Computing Base]]
