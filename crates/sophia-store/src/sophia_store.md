# sophia_store

Persistence and querying of the code graph. Source: `sophia_store.rs` (comments only).

Full design: [[Graph Schema]], with worked queries in [[Query Cookbook]].

## The property that shapes the crate

**The store is not trusted.** Every key is a hash of its own content, so anything can be re-verified by rehashing. A corrupt or hostile store can withhold data or return wrong data, but cannot substitute different content under an existing hash. This removes the largest component from the [[Trusted Computing Base]] at no cost, and it is what makes [[Start Here]]'s "rewrite the core and migrate the database to his own preferred schema" a reasonable invitation rather than a disclaimer.

## Two operations worth noting

`put` is **idempotent** because the key is the content — writing a node that already exists is a no-op. That is not an optimisation, it is the [[Roadmap|M0]] acceptance test: re-ingesting an unchanged corpus must write zero new rows.

`bind` is the **only mutating operation in the system**. Everything else is append-only; names are the single mutable relation ([[Naming and Change Propagation]]).

## Backends

`DuckDbStore` first — two wide tables, a hash primary key, recursive CTEs, no server. The schema is simple enough that a columnar engine handles the hot queries (hash lookup, bounded traversal) very well, and the graph-specific features of [[Property Graph|FalkorDB]] earn their keep only later. `MemStore` is both the hot tier and the test double.

The trait exists so the choice stays reversible. It will leak — query dialects always do — so the leak is confined to per-backend query code.

## The measurement this crate exists to make

Reconstructing one function's IR walks $10^4$–$10^6$ nodes. If that is an order of magnitude slower than re-running the frontend on source text, the architecture must retreat to "core terms persisted, IR regenerated". **This is the single most important empirical question in the project and it should be answered at [[Roadmap|M2]]**, not discovered at M6. Mitigations, in order: chunked subtree blobs (what Git does with packfiles), materialised dependency closure, in-memory hot tier, and finally treating the `Op`/`Instr` layers as an evictable cache.

That last fallback is a genuine retreat from the most ambitious reading of [[Start Here]], and it still keeps everything that matters: core terms, types, equivalences, tests, provenance.

## Related

- [[Graph Schema]] · [[schema]] · [[Query Cookbook]]
- [[Compilation as Query]] · [[Trusted Computing Base]]
- [[Property Graph]] · [[Datalog]] · [[State of the Art - Graph Databases for Code]]
