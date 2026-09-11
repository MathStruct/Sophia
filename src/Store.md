# Store

Thin Julia client over the Rust store. Source: `Store.jl` (comments only).

Schema and design: [[Graph Schema]]; queries: [[Query Cookbook]]; Rust side: [[sophia_store]], [[schema]].

## Deliberately thin

The schema lives in [[schema]] and nowhere else. This module is a `@ccall` surface plus enough Julia sugar to make the REPL a usable place to explore a code graph. Re-declaring the schema here would create a second source of truth — exactly the drift the whole design is meant to prevent.

## `context` is the early demo

The one query [[Start Here]] explicitly predicts: "one could query the exact context of a piece of code". It returns the declaration, its type and effects, its tests, its docs, its callers and callees, its benchmarks, every known alternative implementation with the strength of each claim, and the source span.

Assembling that today takes an IDE, a test runner, a coverage tool and a documentation generator, and still omits the alternatives because nothing records them. It depends on none of the risky machinery — ingestion, hashing and storage only — which makes it the right thing to build early and the right thing to show people. See [[Tests and Documentation as Nodes]] and [[main]].

## Laziness is not an optimisation here

Traversals return lazy iterators. `dependents` on a hot definition can return a very large set and the REPL should not hang because someone typed a hash.

`equivalents` must **never** materialise the transitive closure. Composition unions the `modulo` sets, so long chains are simultaneously enormous and worthless ([[Equivalence and Witnesses]]). Search lazily against the caller's budget, and keep shortest chains.

## Tables.jl conformance

Query results as tables makes the DataFrames and plotting ecosystem available for free. That matters more than it sounds: most of what anyone does with this store in the first year is *measurement* — node counts, query latencies, cache hit rates, store size — and those are the numbers [[Roadmap]] turns on.

## `verify` is a feature, not a debugging aid

Rehashing stored nodes and comparing against their keys is how the store stays untrusted ([[Trusted Computing Base]]). It should be cheap enough to run on a random sample routinely, not something reached for only when something has already gone wrong.

## Related

- [[Graph Schema]] · [[Query Cookbook]] · [[sophia_store]] · [[schema]]
- [[Equivalence and Witnesses]] · [[Trusted Computing Base]]
- [[State of the Art - Graph Databases for Code]]
