#implementation #measurement

> What Sophia physically stores today (`80f71ee` and after), table by table, classified by [[Object Categories]]: authoritative, redundant (an index), derivable (a materialised view), or cache. Measured 2026-10-20 on SQLite. Part of [[Storage and Materialization]].
>
> Sources: `crates/sophia-store/src/sqlite.rs` (schema), `kv.rs` (the key-value layout of redb and LMDB); SQLite's `dbstat` virtual table; corpora as in [[Real Code Corpora]].

## The tables

| SQLite table (KV table) | Holds | Category | Classification |
|---|---|---|---|
| `t_definition` (`Rec`) | per record: id, language, normaliser, kind, component, index, size, **canonical bytes**, **display**, status, **refs**, nominal refs | 2 (and 1 for sophialang) | id, kind, status: **needed**. Canonical bytes: **derivable** for ingested code. Display: **derivable** (labels, variable names: from source). `refs` blob: **redundant** with `t_ref` |
| `t_definition_component`, `_kind` (`Comp`, `Kind`) | indexes | 5 | redundant (indexes) |
| `t_ref` + `t_ref_dst` (`RefRev`) | reference edges, both directions | 2 / 5 | forward: needed by queries (or derivable from the `refs` blob); reverse: **index** |
| `t_nominal_ref` | uses of nominal entities (generic calls) | 2 | needed; derivable for ingested code |
| `t_nominal`, `t_relation` (+ `_member`) | nominal entities and their members (method tables, library roles) | 3 | **authoritative** for authored code; derivable for an ingested library (it is the ingest's grouping) |
| `t_name` (+ `_target`) | names | 3 | **authoritative** (for ingested code: derivable) |
| `t_lineage` (+ `_id`) | lineages | 3 | authoritative |
| `t_annotation` (+ `_subject`) | annotations (`proof` links, roles) | 3 / 4 | authoritative when authored; for Metamath derivable |
| `t_fact` (+ `_subject`) | derived facts with provenance | 4 / 5 | per fact |
| `t_log` | the change log | 3 | **authoritative** (the history) |
| `t_meta` | settings, rendering profiles, **cached query results** (`query-result/<id>`), saved queries | 3 / 5 | profiles and saved queries authoritative; **query results a cache with no eviction** ([[SP-014 The query result cache is never evicted]]) |

**Not stored at all today:** the **source text** of ingested files. For ingested code that is the authoritative object, and everything Sophia stores about it is derived from it.

## Measured: Julia LinearAlgebra (module mode, v6)

Source: 34 files, **1.34 MB** (0.23 MB with zstd -9). Store: **17.7 MB** SQLite, 3 288 records.

| Part | KiB | Share | Class |
|---|---|---|---|
| canonical bytes (in `t_definition`) | 12 036 | 70 % | derivable from source (WP-66 parts, about 9× the source) |
| rest of `t_definition` (ids, kinds, display 25, refs 63) | ≈ 2 670 | 15 % | ids needed; rest derivable or redundant |
| `t_log` | 900 | 5 % | history of the ingest itself |
| method tables (`t_relation` + index) | 468 | 3 % | derivable (ingest grouping) |
| reference edges (`t_ref` + `t_ref_dst`) | 324 | 2 % | needed / index |
| indexes (component, kind, autoindex) | 376 | 2 % | redundant |
| names, nominal, nominal refs, meta, lineage | ≈ 490 | 3 % | mixed |

## Measured: Metamath set.mm

Source: **51.7 MB** (17.0 MB with zstd -9). Store: **567 MiB** SQLite, 98 136 records (48 618 statements, 48 035 proofs, 1 483 syntax).

| Part | MiB | Share | Class |
|---|---|---|---|
| proof canonical bytes | 82 | 14 % | derivable from source (the proofs are authored in the `.mm`) |
| statement canonical bytes | 49 | 9 % | derivable from source |
| `refs` blobs on records | 46 | 8 % | redundant with `t_ref` |
| rest of `t_definition` (ids, components, display, kinds) | ≈ 61 | 11 % | ids needed; rest derivable |
| `t_ref` | 124 | 22 % | needed (or derivable from the blobs) |
| `t_ref_dst` | 115 | 20 % | index |
| `t_log` | 27 | 5 % | ingest history |
| names, lineage, annotations, relations, indexes | ≈ 63 | 11 % | derivable for an ingested library |

## What this says

1. **For ingested code, the whole store is a materialised view of the source.** set.mm: 567 MiB derived from 17 MB of compressed authority. A store that kept only the source blobs, the ids, the kinds and one copy of the edges would hold everything a query needs and could rebuild the rest. Rough estimate for set.mm: 17 MB of source, 98 136 × 32 B ids ≈ 3 MB, 1.5 M edges as 8-byte surrogate pairs (SP-011 option 1) ≈ 24 MB and a reverse index of the same size: **under 70 MB, against 567 MiB.** To be measured, not assumed ([[Storage Measurements]] M-1, M-2).
2. **References are stored three times** (blob, forward table, reverse index). One copy plus an index is enough (SP-011 options 2 and 4).
3. **The canonical form of WP-66 is large.** 9× the source for Julia. It is fine as a hashing input and wasteful as storage. If it is kept at all, it should be compressed or rebuilt.
4. **The log records the ingest itself.** For a deterministic ingest (source, frontend version) one log entry naming the source blob and the frontend would replace thousands of entries: the ingest is a **transformation record** (SPEC-006 §6a), not a series of edits.
5. **Query results accumulate forever** in `t_meta` ([[SP-014 The query result cache is never evicted]]).
6. **Not all of this is waste.** Rebuilding the canonical bytes of one definition requires parsing its file. If Moonkale shows a definition's canonical form often, keeping it (compressed) may be right. That is a measured decision, per [[Materialization Policy]].

## What is not changed yet

Nothing in the storage layout has been changed on the strength of this page (SR-126). The first change, when Daniel agrees, would be the cheapest and least risky one: drop the `refs` blob where an edge table exists (SP-011 option 2), then measure the rest.
