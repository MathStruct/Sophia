"""
    Sophia.Store

Thin Julia client over the Rust store.

Design notes: `Store.md`, `markdown/Design/Graph Schema.md`,
`markdown/Design/Query Cookbook.md`.
COMMENTS ONLY — nothing here is implemented.

Deliberately thin. The schema lives in `crates/sophia-store/src/schema.rs` and
nowhere else; this module is a `@ccall` surface plus enough Julia-idiomatic
sugar to make the REPL a usable place to explore a code graph. Re-declaring the
schema here would create a second source of truth, which is exactly the drift
the store is designed to prevent.

# Intended API

    open(path; backend=:duckdb, readonly=false) -> Store
    close(s::Store)

    Base.getindex(s::Store, h::Hash)          -> Node              # s[h]
    Base.haskey(s::Store, h::Hash)            -> Bool
    put!(s::Store, n::Node)                   -> Hash              # idempotent

    children(s, h)        ; parents(s, h, kind)
    dependents(s, h; depth=nothing)
    subtree(s, h; budget)

    resolve(s, ns, sym; at=now_world())       -> Union{Hash,Nothing}
    bind!(s, ns, sym, h)                                           # the ONLY mutation

    equivalents(s, h; level=:rewrite, modulo=Symbol[], maxchain=3)
    context(s, h)                             -> NamedTuple

# `context` is the demo

    context(s, h) -> (
        decl, type, effects,
        tests   = [...],
        docs    = [...],
        callers = [...], callees = [...],
        equivalents = [(hash=..., level=..., modulo=..., witness=...), ...],
        source  = (file=..., line=...),
        benchmarks = [...],
    )

This is the query `markdown/Start Here.md` predicts: "one could query the exact
context of a piece of code". Today that answer needs an IDE, a test runner, a
coverage tool and a doc generator, and still omits the alternative
implementations because nothing records them. It depends on none of the risky
machinery — just ingestion, hashing and storage — which makes it the right
thing to build early.

# Iteration and laziness

Traversals return lazy iterators, not materialised vectors. A `dependents`
query on a hot definition can return a very large set, and the REPL should not
hang because someone typed a hash.

`equivalents` must NEVER materialise the transitive closure: composition unions
the `modulo` sets, so long chains are both enormous and worthless
(`markdown/Design/Equivalence and Witnesses.md`). Search lazily against the
caller's budget.

# Tabular interface

`Tables.jl` conformance on query results makes the whole DataFrames/Plots
ecosystem available for free, which matters because most of what anyone does
with this store early on is measurement: node counts, query latencies, cache
hit rates, store size (`markdown/Design/Roadmap.md`).

# Verification

    verify(s::Store; sample=nothing) -> VerifyReport

Rehash stored nodes and compare against their keys. The store is not trusted
(`markdown/Design/Trusted Computing Base.md`), so this is a first-class
operation rather than a debugging aid, and it should be cheap to run on a
random sample.
"""
module Store
end
