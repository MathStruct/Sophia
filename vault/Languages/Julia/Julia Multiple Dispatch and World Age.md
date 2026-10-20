#design #julia

> Three things that must not be confused: a **generic function's identity**, its **method table at a world**, and a **dispatch result** for given argument types at a world. Sophia stores the first two and does not yet compute the third. Raised by the external review of 2026-10-20 (§5, SR-130); specified in [[SPEC-006 Worlds, Artifacts and Evidence]] §1a.
>
> Sources: Bezanson et al., *Julia: Dynamism and Performance Reconciled by Design*, OOPSLA 2018; Belyakova et al., *World Age in Julia*, OOPSLA 2020; Zappa Nardelli et al., *Julia Subtyping: A Rational Reconstruction*, OOPSLA 2018; [[Multiple Dispatch]]; [[World Age]]; [[SP-006 Open method tables versus content addressing]].

## The three objects

| Object | Notation | What it is | In Sophia |
|---|---|---|---|
| **Generic identity** | g | a binding: (module, name). Stable while methods come and go | a nominal entity (`jl.Base.show`), fresh id |
| **Method table** | MT(g, w) | the set of g's methods in world w | `Members(g, v)`, versioned (SPEC-006 §1); frozen in a world snapshot (§2) |
| **Dispatch result** | dispatch(g, argtypes, w) | the most specific applicable method, or an ambiguity or a `MethodError` | **not computed**; would be a derived fact citing w |

Content ids only ever cover the first: a caller's id includes "calls g", never "calls method m of g". Adding a method changes MT, which changes w, which may change dispatch results. It never changes any content id. That is SP-006's answer to the invalidation storm.

## Why a static tool cannot simply compute dispatch

- **Argument types are runtime values** in general. Julia's compiler infers them per call site and per specialisation; tree-sitter does not.
- **Subtyping** with `where`, `Union`, `Vararg` and diagonal rules is the hardest part of Julia's type system (Zappa Nardelli et al.).
- **The world moves:** `eval`, `include` at run time and package loading add methods; code that runs in an older world does not see them (Belyakova et al.).
- **Methods of other packages** take part: dispatch for `*` on a LinearAlgebra type may land in SparseArrays.

## What Sophia can say honestly

| Claim | Level | Status |
|---|---|---|
| "f calls the generic function g" | L3 | yes (generic uses, `nominal_refs`) |
| "m is a method of g" | L3 | yes, 94.5 % agreement with Julia ([[Julia Conformance]]) |
| "in world w, g has these methods" | L3 | yes, via world snapshots, for the methods Sophia ingested |
| "a call g(x::T) in world w reaches m" | L5-adjacent | **no**; would come from Julia itself (below) |
| "f's compiled code is valid in world w′" | — | no; Julia's backedges know, Sophia does not |

## The way forward: ask Julia, record the answer

Rather than reimplementing dispatch, let Julia answer and Sophia **record** the answer as evidence:
- run Julia with the package loaded in a known environment (Manifest, Julia version: part of w);
- for chosen call signatures, `which(g, Tuple{T…})` gives the method; `methods(g)` gives the table; `Base.method_instances` and backedges give what compiled code relied on;
- store each answer as a **dispatch fact** citing (g, argument types, world, Julia version), with `tested` trust: it is an observation of one implementation;
- when MT(g) changes, exactly the dispatch facts for g whose signatures overlap the new method's are stale (SR-85's minimal invalidation).

This is the same pattern as the Metamath check (an external reference implementation as the oracle) and the [[Julia Conformance]] check. It is experiment E-3 in [[Semantic Boundary Experiments]].

## Consequences for caching

Anything compiled from Julia code (a specialisation, a derivative, a Reactant executable) depends on dispatch results, so its artifact id A must include the world (SPEC-006 §3). Two builds of the same source in different worlds are different artifacts, and that is correct: they may run different methods.
