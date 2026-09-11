# Precompile

The content-addressed code cache. Source: `Precompile.jl` (comments only).

Full design: [[Content-Addressed Precompilation]]. Milestone: [[Roadmap|M3]].

## Why this module is the one to build first among the ambitious ones

[[Start Here]] ends with it: "I wanted to improve precompilation for Julia. This I wanted to solve with a graph database." It has the clearest success criterion, the shortest path to being useful, and it needs **none** of the cross-language machinery, dependent types, or a new language. It can be validated entirely on its own — which makes it the part of the project that could justify the rest to someone who does not already believe in it.

## The actual claim, stated precisely

`compile(h_run, h_target, h_opts) → machine code` is pure, so the result is cacheable forever, shareable across machines with verification by rehashing, and invalidated exactly.

But the win is **not caching** — [[Pkgimage|pkgimages]] already cache native code. The win is **precision of invalidation**, and precision requires persisting dependency facts that Julia currently holds only in memory as backedges and discards at process exit.

Concretely: each specialisation records `DispatchFact` nodes ("at this call site, with these argument types, the applicable method was `m`"). Adding a method then invalidates exactly the specialisations whose recorded facts it contradicts, rather than everything that transitively touched the function. See [[World Age]] and [[Multiple Dispatch]].

The use of the *erased* hash `h_run` ([[Hashing and Identity]]) is the other half: changes that do not survive erasure — a docstring, a renamed type parameter, an annotation that was already inferred — do not invalidate compiled code. Today they do, because invalidation is at file and package granularity.

## `:supplement` is the only responsible default

Consult the store first, fall back to normal Julia compilation. It can be switched off, it cannot make anything wrong, and it makes the experiment safe to run against a real workload — which is what generating credible numbers requires.

## The integration points are fragile, and should be treated as such

Method-table overlays, `--trace-compile` collection, external abstract-interpreter hooks, and the pkgimage loading path are all internals. They change between Julia releases and none is a supported extension point. Each belongs behind a small shim with a narrow, explicit compatibility range rather than an optimistic one.

## The four numbers

Time-to-first-execution (cold / warm / stock), specialisations retained after a method addition vs stock invalidation, cache hit rate across two projects sharing dependencies, and store size plus fetch latency.

They are obtainable at M3, long before any cross-language work. **If they come out flat, that is important information about the whole architecture and it is much better to have it early** — which is exactly why [[Roadmap]] puts this milestone before anything about equivalence.

## Related

- [[Content-Addressed Precompilation]] · [[Pkgimage]] · [[World Age]]
- [[Hashing and Identity]] · [[Frontend]] · [[Store]]
- [[State of the Art - Julia Compilation and Precompilation]] · [[State of the Art - Incremental Computation]]
