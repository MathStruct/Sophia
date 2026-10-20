#design

> Shrinking what is kept, without touching identity. Part of [[Storage and Materialization]]. 2026-10-20. The theory note is [[Compression and Canonical Form]]; this page is the practical list, ordered by expected payoff on real data ([[Current Inventory]]).
>
> Sources: [[SP-011 Reference edges dominate storage on real code]]; [[Block Size and Hash Overhead]]; [[Zero-Copy Storage of Canonical Graphs]]; zstd dictionary compression (Collet, RFC 8878).

Compression is a **physical encoding** (SPEC-006 §6): it may change freely and never changes an id. Verification re-derives canonical bytes and re-hashes, so whatever is stored must reproduce them exactly.

| # | Technique | Applies to | Expected gain | Cost | Status |
|---|---|---|---|---|---|
| 1 | **Do not store derivable bytes** (rebuild from source) | WP-66 canonical bytes, Metamath record bytes, display data | the largest: 70 % of the LinearAlgebra store, about a quarter of set.mm's | a parse on demand | proposed ([[Current Inventory]]) |
| 2 | **One copy of each edge** (drop the `refs` blob where an edge table exists) | reference edges | 8 % of set.mm | rebuild a record's refs by an index scan | proposed (SP-011 option 2) |
| 3 | **Surrogate keys** (8-byte local ids in edge tables) | edges and indexes | ≈ 75 % of edge storage, 30 % of set.mm | an id map; translation at the boundary | proposed (SP-011 option 1) |
| 4 | **Source blobs, zstd** | authored source | 3–6× (set.mm 51.7 → 17.0 MB; LinearAlgebra 1.34 → 0.23 MB, zstd -9) | decompression on read | proposed |
| 5 | **Dictionary compression** across similar small records | canonical bytes if kept, proofs | further 2–3× on small records (typical for zstd dictionaries) | dictionary management | idea |
| 6 | **Hash-consed subterms** | proof terms, core terms | large for proofs (Metamath's own compressed format reuses steps) | a term graph instead of trees | idea; the core calculus is already a DAG by content |
| 7 | **Page-level compression** in the backend | everything | modest | backend support (SQLite has none built in; DuckDB compresses columns) | out of scope (SR-105) |

The order to try them is 2, 1, 3: cheapest and safest first, each measured before the next ([[Storage Measurements]]).
