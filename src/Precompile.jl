"""
    Sophia.Precompile

The content-addressed code cache — the original motivation of the project.

Design notes: `Precompile.md`,
`markdown/Design/Content-Addressed Precompilation.md`.
COMMENTS ONLY — nothing here is implemented.

markdown/Start Here.md ends with: "Also I wanted to improve precompilation for
Julia. This I wanted to solve with a graph database." Of everything in the
project this has the clearest success criterion, the shortest path to being
useful, and — importantly — it needs NONE of the cross-language machinery,
dependent types, or a new language. It can be validated on its own.

# The reformulation

    compile(h_run, h_target, h_opts) -> machine code

All three inputs are content hashes, so the function is pure and the result is:

  * cacheable forever
  * shareable across projects, users and machines — with verification by
    rehashing rather than by trust
    (`markdown/Design/Trusted Computing Base.md`)
  * invalidated EXACTLY: only entries whose `h_run` changed

`h_run` is the ERASED-term hash (`markdown/Design/Hashing and Identity.md`).
That matters: changes that do not survive erasure — a docstring, a renamed type
parameter, an annotation that was already inferred — do not change `h_run` and
therefore do not invalidate compiled code. Today they do, because invalidation
is at file and package granularity.

# Why the invalidation cascade shrinks but does not vanish

A specialisation's correctness depends on the method table it was compiled
against: if a more specific method appears, devirtualised call sites are wrong
(`markdown/Wiki/Julia/World Age.md`). So the key is honestly:

    h_run = hash(erased term) ⊕ hash(the dispatch decisions it baked in)

Each specialisation records `DispatchFact` nodes — "at this call site, with
these argument types, the applicable method was m". Adding a method then
invalidates exactly the specialisations whose recorded facts it contradicts,
instead of everything that transitively touched the function.

THIS IS THE ACTUAL CLAIM OF THIS MODULE, and it is worth stating precisely:
the win is not caching — `pkgimages` already cache
(`markdown/Wiki/Julia/Pkgimage.md`) — the win is PRECISION OF INVALIDATION,
and precision requires persisting dependency facts that Julia currently holds
only in memory as backedges and discards at process exit.

# Intended API

    enable_cache!(store; mode=:supplement)   # :supplement | :exclusive | :record
    cache_stats()                            -> hit rate, size, evictions
    warm(store, mod::Module)                 # precompute for a package
    explain(mi::Core.MethodInstance)         # why did this miss?

`:supplement` consults the store first and falls back to normal Julia
compilation. That is the only responsible default — it can be switched off, it
cannot make anything wrong, and it makes the experiment safe to run on a real
workload.

# Integration points (all of them fragile)

  * `Base.Experimental.@overlay` / method-table overlays
  * precompile statement collection (`--trace-compile`)
  * `Core.Compiler` external abstract-interpreter hooks
  * the pkgimage loading path

These are internals. They change between Julia releases and none of them is a
supported extension point. This module should isolate every one of them behind
a small shim, and the version compatibility range should be narrow and
explicit rather than optimistic.

# The numbers that would justify the architecture

  1. time-to-first-execution: cold cache vs warm cache vs stock Julia
  2. fraction of specialisations retained after adding a method to a loaded
     package, vs stock invalidation
  3. cache hit rate across two different projects sharing dependencies
  4. store size per package; latency to fetch one specialisation

These are obtainable at M3, long before any cross-language work, and they are
what would convince someone who does not already believe in the idea.
If they come out flat, that is important information about the whole project
and it is worth having early.
"""
module Precompile
end
