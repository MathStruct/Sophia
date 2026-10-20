#design #julia

> How Julia definitions and sophialang definitions relate in one store: what can point at what, what "the same" may mean between them, and what lowering would require. Mostly design; little is built. 2026-10-20.
>
> Sources: [[SPEC-002 Sophialang Principles]] (Julia-flavoured syntax; dispatch, SR-59); [[SPEC-005 Rendering Profiles]]; [[Equivalence and Witnesses]]; [[Cross-Language Semantic Hazards]]; [[S-ADR-0007 The store is language-agnostic]].

## Two different things called "Julia" in Sophia

1. **Julia the stored language**: records of language `julia` with Moonkale's WP-66 ids, ingested from `.jl` files. Their semantics are Julia's, and Sophia knows them only as far as [[Julia Overview]] says.
2. **The Julia profile of sophialang**: sophialang definitions printed and parsed in a Julia-like syntax ([[SPEC-005 Rendering Profiles]]). Their semantics are sophialang's. They are not Julia code and do not run in Julia.

A Julia-profile sophialang function and a Julia function with the same text have **different ids** (different language, different normaliser) and possibly different meanings (integer overflow, dispatch, mutation). That is deliberate (S-ADR-0007).

## Relations between them

| Relation | Meaning | Evidence | Status |
|---|---|---|---|
| reference | a sophialang definition names a Julia generic function or type as a foreign entity | none needed: it is a link | possible today through nominal ids; no elaborator support |
| `equivalent` | for all inputs in a stated domain, they compute the same result | `tested` (differential runs), `checked` (proof) | specified (SPEC-006 §6a), not built |
| `refines` | the sophialang definition is a specification the Julia code satisfies | proof or tests | specified only |
| `lowers-to` | a Julia definition translated into sophialang core | a translator with stated coverage | not started |

## What lowering Julia into sophialang would need

Level L4 of [[Languages]]. Only for a fragment, never for "Julia":
- a pure, first-order, statically dispatched fragment (arithmetic, loops, arrays without aliasing) could lower to sophialang core;
- multiple dispatch lowers only when the dispatch result is known (a recorded dispatch fact for a fixed world);
- mutation and aliasing need sophialang's effect and region types ([[Effects Memory and Resources]]);
- macros and generated functions lower only after expansion;
- every lowered definition is a transformation output with an obligation (`eval_julia(f, x) = eval_sophia(lower(f), x)` on the fragment's domain), backed by differential tests first.

This is the route to Daniel's eventual goal of Lenticulum running in Sophia (SR-115), and it is long. The AD experiments come first because they show which derived programs and artifacts need to exist before any of this is useful ([[Semantic Boundary Experiments]]).
