# Content-Addressed Precompilation

[[Start Here]] ends with the origin story: "I wanted to improve precompilation for Julia. This I wanted to solve with a graph database." Of everything in this project, this is the goal with the clearest success criterion and the shortest path to being useful, so it deserves a precise statement.

## The problem

Julia compiles per *method specialisation* — one native implementation per concrete argument-type tuple, generated on demand ([[State of the Art - Julia Compilation and Precompilation]]). This gives excellent steady-state performance and the well-known latency problem. Since 1.9, `pkgimages` cache native code per package, which helps a great deal but is keyed at package granularity with invalidation at roughly that granularity too. The remaining pain:

- **Invalidation cascades.** Loading a package that adds a method can invalidate previously compiled code that assumed the old method table ([[World Age]]). One new method can discard a lot of work.
- **Package granularity.** Editing one function in a package invalidates the package image, not just that function's dependents.
- **No sharing across projects.** Two projects using the same package at the same version compile the same specialisations independently.
- **No sharing across machines.** There is no notion of "someone already compiled exactly this".

## The content-addressed reformulation

Define the code cache as a pure function

$ "compile" : (h_"run", h_"target", h_"opts") ↦ "machine code" $

where `h_run` is the erased-term hash from [[Hashing and Identity]], `h_target` identifies the triple/CPU features/datalayout, and `h_opts` the optimisation settings. Because all three are content hashes, the result is:

- **cacheable forever** — inputs fully determine output,
- **shareable** — across projects, users and machines, with verification by rehashing rather than by trust ([[Trusted Computing Base]]),
- **invalidated exactly** — a change invalidates precisely the entries whose `h_run` changed, which is precisely the transitive dependents whose *erased core term* changed.

The word "erased" is doing real work. Changes to type-level information that does not survive erasure — a docstring, a renamed type parameter, a type annotation that was already inferred — do not change `h_run` and therefore do not invalidate compiled code. Today they do, because invalidation is at file/package granularity.

## Why the invalidation cascade shrinks but does not vanish

Julia's dynamism means a specialisation's correctness depends on the method table it was compiled against: if a more specific method appears, previously devirtualised call sites become wrong. That dependency is real and must be part of the key. The honest formulation makes it explicit:

$ h_"run" "depends on" quad ("the term") ⊕ ("the set of dispatch decisions it baked in") $

i.e. each specialisation records the *method-table queries it relied on* — "for this call site with these argument types, the applicable method was `m`" — as `DEPENDS_ON` edges to a `DispatchFact` node. Adding a method invalidates exactly the specialisations whose recorded dispatch facts it contradicts, rather than everything that transitively touched the function. This is a graph query, and it is strictly more precise than what Julia's backedge mechanism can express today, because the facts are persisted and inspectable rather than being in-memory backedges discarded at process exit.

That is the concrete, defensible claim of this note: **the win is not caching, it is precision of invalidation, and precision requires persisting the dependency facts that the current design throws away.**

## Interaction with the rest of the design

- Needs [[Hashing and Identity]] (the three-hash scheme), and nothing else from the ambitious parts of the project.
- Does **not** need cross-language equivalence, dependent types, or a new language. It is the part of the idea that can be validated on its own.
- Fits alongside `pkgimages` rather than replacing them: the store can be a supplementary cache consulted before falling back to normal compilation.
- The same construction gives a shared build cache for any frontend, so the mechanism generalises for free.

## Measurable success criteria

1. Time-to-first-execution for a fixed workload, cold cache vs. warm cache vs. stock Julia.
2. Fraction of specialisations retained after adding a method to a loaded package, vs. stock invalidation.
3. Cache hit rate across two different projects sharing dependencies.
4. Store size per package, and query latency to fetch one specialisation.

These are the numbers that would justify the whole architecture, and they are obtainable long before any of the cross-language machinery exists. They belong at [[Roadmap|M2–M3]], not at the end.

## Prior art worth copying rather than reinventing

- `pkgimages` and `PackageCompiler.jl` for the artifact format and loading mechanics.
- Nix and Bazel for remote content-addressed build caches ([[State of the Art - Incremental Computation]]).
- `ccache`/`sccache` for the pragmatics of a compiler cache that has to be faster than the compiler.
- Unison for never needing to recompile a definition that already exists by hash.

## Related

- [[State of the Art - Julia Compilation and Precompilation]]
- [[Hashing and Identity]]
- [[World Age]]
- [[Multiple Dispatch]]
- [[Pkgimage]]
- [[State of the Art - Incremental Computation]]
- [[Precompile]] — the Julia module that would implement this
