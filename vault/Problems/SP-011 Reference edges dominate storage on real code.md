#open-problem

> **Status: open (2026-10-20).** Related: [[Block Size and Hash Overhead]], [[Benchmark Results]] (B-16), [[Real Code Corpora]], SPEC-001 §12.

**Problem.** On Metamath's set.mm, 1.5 M reference edges (about 31 per proof) take 42 % of a 564 MiB SQLite store. They are stored three times: the `t_ref` table, its reverse index, and a refs blob on every record. Each edge is two 32-byte ids. On synthetic code (3 references per definition) this was invisible, and H3 ("local ids save little") was refuted there. **Real code is reference-dense; synthetic code was not.**

**Options.**
1. **Surrogate keys:** an 8-byte local id per record, used in the edge tables and indexes only. Content ids stay the identity, and edges translate at the boundary. This saves about 75 % of edge storage. The cost is an id map, and lookups by content id go through it.
2. **Drop the refs blob on the record** where the edge table exists (SQLite, DuckDB). The record can be rebuilt from the edge table.
3. **Prefix-compressed keys** in the key-value backends. LMDB has no prefix compression; redb's B-tree does not either. A custom page layout is outside the "no new engine" rule (SR-105).
4. **Edges grouped per source:** one row per record listing its references (like the refs blob) plus a reverse index only. That halves the forward cost; the reverse index stays.

**Proposal.** Option 2 now (cheap), and option 1 measured as an experiment. It is a storage change, not an identity change, so ids are unaffected. Daniel has said large changes are fine.
