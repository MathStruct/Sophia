#measurement #storage #proposal

> M-1 and M-2 of [[Storage Measurements]], measured 2026-11-20 (B-033). A store that keeps only sources, the manifest, ids, kinds and one copy of the edges is **20× smaller for Julia LinearAlgebra and 10× smaller for set.mm**. It still rebuilds every record with the same ids and edges. Rebuilding canonical bytes costs 23 µs per Julia definition once the module is resolved. But resolution is module-wide, so showing one definition's canonical form without stored state takes the whole module: 0.82 s for LinearAlgebra, 2.2 s for set.mm. That is over the 50 ms budget. Kept compressed in blocks, the canonical bytes are small: 0.23 MiB for LinearAlgebra, 27 MiB for set.mm. The proposal at the end is for Daniel. **Nothing in the store has been changed** (SR-126).
>
> Sources: [[Review 2026-11-20 Codex B-033 (external)]]; code: `crates/sophia-ingest/examples/lean_layout.rs` ([[lean_layout]]); `crates/sophia-store/src/sqlite.rs` (the schema measured); [[Current Inventory]] (the 2026-10-20 baseline); [[SP-011 Reference edges dominate storage on real code]]; [[Compression]].

## How it was measured

```sh
cargo build --release -p sophia-cli -p sophia-ingest --example lean_layout
C=$SOPHIA_CORPORA/m1
sophia --store $C/la.sqlite ingest-code julia LinearAlgebra <stdlib v1.13>/LinearAlgebra/src   # 1.64 s
sophia --store $C/mm.sqlite ingest-mm $SOPHIA_CORPORA/set.mm/set.mm                          # 46 s
lean_layout $C/la.sqlite $C/la-lean > $C/la-lean.txt
systemd-run --user --wait -p MemoryMax=12G lean_layout $C/mm.sqlite $C/mm-lean > $C/mm-lean.txt   # 105 s, 633 MB peak
```

Both stores are fresh ingests at `09b7025`, made with Julia 1.13.1's LinearAlgebra (34 files, 1.34 MB) and set.mm (51.7 MB). Each layout is a copy of the one before, transformed with SQL and `VACUUM`ed ([[lean_layout]] has the steps). Sizes are file sizes in MiB. The full outputs, with `dbstat` per table, are in `$SOPHIA_CORPORA/m1/*-lean.txt`.

## M-1: the ladder

| Step | What is removed or changed | LinearAlgebra | × | set.mm | × |
|---|---|---|---|---|---|
| L0 | nothing (vacuumed copy) | 19.34 | 1.00 | 575.18 | 1.00 |
| L1 | `refs` and `nominal_refs` blobs (edges stay in their tables) | 19.21 | 1.01 | 522.09 | 1.10 |
| L2 | canonical bytes and display of every rebuildable record | 5.38 | 3.59 | 348.39 | 1.65 |
| L3 | integer keys for edges, a string dictionary, component only for SCCs | 4.70 | 4.11 | 163.96 | 3.51 |
| L4 | ingest log as one entry; sources `zstd -19` | 2.54 | 7.62 | 101.44 | 5.67 |
| L5 | floor: ingest-derived names, lineage, annotations, method tables, facts | 0.95 | 20.30 | 56.18 | 10.24 |

**Verified.** L4 and L5 are each checked from their own contents. The program decompresses the sources and rebuilds every record with the frontend. It also rebuilds the manifest and source records themselves.
- **The same records:** every rebuilt record is still held, and nothing else is. LinearAlgebra: 3 284 definitions plus 35 sources and manifest. set.mm: 98 136 plus 2.
- **The same fields:** each record has the same language, normaliser, kind, status, index, component size, multi-member component and outgoing edges.
- **Nothing lost in the dropped payloads:** the rebuilt canonical bytes and display equal L0's, for every record.
- **The same nominal edges** as L0's.
- **The ids:** for set.mm the rebuilt bytes hash to the stored ids (`Record::verify`, all 98 138). For Julia the ids come from Moonkale's crate, so for those the id equality is the check.

Four damaged copies of LinearAlgebra's L5 or L0 were each refused:
- one manifest-to-source edge deleted;
- one display changed in L0;
- one nominal edge deleted;
- a second manifest added.

Rebuild time from L5: 0.84 s and 2.31 s.

**Where the bytes go:**
- **Canonical bytes** dominate Julia: L2 removes 72 % of the file.
- **Edges** dominate set.mm: 1 520 873 edges took 205 MiB in L2, forward and reverse at about 141 B per edge. As integer pairs they take 31.7 MiB, about 22 B per edge (6.5× less). That is SP-011's option 1 and B-034.
- **The log of the ingest**: 28 MiB of set.mm (295 056 entries) and 1.1 MiB of LinearAlgebra.
- **Ingest-derived metadata** in L4:
  - set.mm: names, lineage, annotations and roles, about 45 MiB. They are still keyed by 32-byte ids; integer keys for them are not measured.
  - LinearAlgebra: the source-span facts take 1.0 MiB, the largest table left.

**Hypotheses.** M-1 expected set.mm under 70 MB and LinearAlgebra under 2 MB, and was to fail below a 3× saving. The floor L5 confirms both: 58.9 MB and 1.0 MB. The floor cannot answer queries by name, though. The usable layout L4 saves 5.7× and 7.6×, still above 3×.

## The alternative: keep canonical bytes, compressed

What L2 drops, compressed with `zstd -19`:

| Corpus | Raw | Blocks of 256 records | One stream |
|---|---|---|---|
| LinearAlgebra (3 284 records) | 11.78 MiB | **0.23 MiB** (51×) | 0.17 MiB (68×) |
| set.mm (98 136 records) | 140.07 MiB | **26.77 MiB** (5.2×) | 16.57 MiB (8.5×) |

The sizes leave out the framing and offsets a real block needs to find one record in it. That would be about 4 to 8 bytes per record: under 0.8 MiB for set.mm. WP-66's encoding is very repetitive. Compressed in blocks, LinearAlgebra's canonical bytes take about as much as its compressed source. Reading one definition would decompress one block, about 0.9 MB raw for Julia. **Expected, not measured:** about a millisecond.

## M-2: what rebuilding costs

**Julia LinearAlgebra.** Medians of 5 runs; 3 416 definitions, 3 284 distinct records.

| Operation | Time |
|---|---|
| whole rebuild (`module::ingest`) | 820.5 ms (240 µs per definition) |
| … parsing all 34 files | 205.9 ms |
| … module resolution and ids (`julia::module`) | 333.7 ms |
| parse one file | median 4.0 ms, p90 11.1 ms, max 43.5 ms |
| WP-66 parts of one definition (`definition`), references given | median 22.3 µs, p99 297 µs, max 854 µs |
| one definition with a reference sidecar: parse its file, then encode | median 7.3 ms, p99 43.7 ms, max 44.0 ms (an estimate: the sum of the two measured parts; it leaves out reading the sidecar and `to_record`'s replacement of references by ids) |

**Metamath set.mm.** The median of 3 runs: `metamath::parse` takes 2.24 s for 99 242 statements and proofs (22.6 µs each). One statement needs the frames in scope at its position, so today a rebuild reads the file up to it.

**Hypothesis.** M-2 expected under 1 ms per Julia definition ("parse the file once, normalise one definition"). It was to fail if one Moonkale "show canonical form" request exceeded 50 ms.
- **Encoding alone confirms it:** 22 µs median, at most 0.9 ms.
- **The premise is wrong.** One file is not enough to rebuild a Julia definition. Its references are resolved across the whole module (SPEC-008), and they enter its bytes.
- **So the failure criterion is met.** A request with no stored state costs a full module rebuild: 0.82 s for LinearAlgebra, 2.2 s for set.mm.
- **Two ways back under budget:**
  - a per-definition sidecar of resolved references: median 7.3 ms, at worst 44 ms, set by the largest file's parse;
  - an in-memory cache filled by the first request. It costs one rebuild per module and session.

## Proposal for Daniel (not decided, nothing changed)

Ordered by gain per risk. Each step is a physical change only: no id changes (SPEC-006 §6), and each is followed by `reconstruct`.

1. **Integer keys for edges** (L3, B-034): set.mm −184 MiB (173 of it the edges), the largest single gain. The cost is the views and the four backends' edge code. The KV backends would need the same change or a documented difference.
2. **Keep canonical bytes, compressed in blocks, instead of dropping them** (instead of L2): −11.6 MiB for LinearAlgebra, −113 MiB for set.mm. "Show canonical form" stays cheap, with no sidecar and no cache to keep in step. Dropping them outright only pays where nobody reads them.
3. **Drop the `refs` and `nominal_refs` blobs** (L1): −9 % for set.mm, −1 % for LinearAlgebra. Small, but it removes a third copy that can disagree with the tables. The store would read references from `t_ref`.
4. **The ingest as one transformation record in the log** (SPEC-006 §6a): −28 MiB for set.mm.
5. **Compress sources** ([[Compression]] item 4): −35 MiB for set.mm.

Estimated layout with 1 to 5 (L4 plus 26.8 MiB of compressed canonical bytes; a sum, not measured as one layout): set.mm about **128 MiB, 4.5×**; LinearAlgebra about **2.8 MiB, 7×**.

Not proposed:
- **dropping names, lineage and annotations** (L5): queries need them;
- **a reference sidecar** for Julia: compressed canonical bytes make it unnecessary.

**Open measurements:**
- query speed on L3's integer-keyed edges (B-17's queries, against L0);
- one block's decompression time;
- integer keys for the other id-keyed tables.
