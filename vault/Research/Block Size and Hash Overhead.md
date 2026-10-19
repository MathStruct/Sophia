#design #comparison

> Given the processor's caches, what is the right **block size** (the unit that is hashed, deduplicated, stored and compiled), and what **hash-to-data ratio** is efficient (SR-71)? **The analysis below gives a hypothesis, and the benchmarks decide:**
> - The identity unit is the **definition**.
> - Inside a definition, share a subterm only if its canonical size is at least **T ≈ 64–256 bytes** (1–4 cache lines).
> - Store references as **local 8-byte ids**, not 32-byte digests. The digests are still used for hashing.
> - Lay terms out in memory as **flat node arrays**, so that a typical definition fits in L1.
>
> With those choices the hash overhead stays around 5–15 % of the data, rather than the 200–400 % of hashing every node.
>
> Sources: Hennessy and Patterson, *Computer Architecture*, 6th ed. (memory hierarchy); Drepper, *What Every Programmer Should Know About Memory*, 2007; O'Connor et al., *BLAKE3* specification, 2020 (64-byte blocks, 1 KiB chunks); Muthitacharoen, Chen and Mazières, *A Low-bandwidth Network File System* (content-defined chunking), SOSP 2001; Unison's SQLite codebase schema (an integer `object_id` per hash); flat ASTs (Sampson, *Flattening ASTs*, 2023; Zig's and Carbon's compilers). code: `crates/sophia-bench` (B-01 to B-08).

## 1. The hardware numbers that matter

Typical current desktop and server parts. They are *orders of magnitude*, to be checked on the benchmark machines:

| Level | Size | Latency | Line or page |
|---|---|---|---|
| L1 data | 32–64 KiB per core (Apple: 128 KiB) | ~4–5 cycles (~1 ns) | 64 B line (Apple L2: 128 B) |
| L2 | 0.5–2 MiB per core (Apple: 4–16 MiB shared) | ~12–15 cycles | 64 B |
| L3 | 8–64 MiB shared | ~40–60 cycles (~10–15 ns) | 64 B |
| DRAM | GiBs | ~80–120 ns | page 4 KiB (Apple 16 KiB); huge pages 2 MiB |
| NVMe SSD | TiBs | ~10–100 µs | 4 KiB |

BLAKE3 works on 64-byte blocks inside 1 KiB chunks. Hashing *n* bytes costs a fixed setup plus about ⌈n/64⌉ compressions, so tiny inputs are dominated by the setup. A 32-byte digest is half a cache line.

## 2. Granularity: what one hash covers

Let *s* be the average canonical size of a hashed unit and *r* the number of references it contains:
- each unit costs one 32-byte digest as its key;
- each reference costs 32 bytes inside the canonical bytes, or 8 bytes if stored as a local id.

| Unit | Typical *s* | Key overhead | Reference overhead (32 B each) | Verdict |
|---|---|---|---|---|
| **every term node** (as in full Merkle hashing of ASTs) | 8–16 B | **200–400 %** | each child is a reference: another ~200 % | too fine: twice to five times the data, and a hash per node per edit |
| **every subterm ≥ T** (threshold sharing) | ≥ T | ≤ 32/T: 50 % at T = 64, 12.5 % at T = 256 | only for shared children | candidate for deduplication *inside* the store |
| **definition** (Unison, Moonkale WP-66, Sophia v0) | 200 B – 4 KiB (hypothesis; B-07 measures it) | 1–15 % | *r* × 32 B: 10 calls are 320 B, which is significant | **the identity unit** |
| **module or file** | 10–100 KiB | < 0.1 % | small | too coarse: one edit invalidates everything, and dedup is poor |

Conclusions:
1. **The definition is the identity and compilation unit.** It is the smallest unit people change independently, the one dependency tracking needs, and the right size for caches: 200 B – 4 KiB is 3–64 cache lines, within L1.
2. **Hashing per node is wasteful** for identity. Deduplication of *large* repeated subterms inside definitions is a storage optimisation with a threshold *T* (to be measured: B-07).
3. **References dominate the overhead,** not keys. A definition with 10 references carries 320 bytes of digests, often as much as its own structure. So:
   - **canonical bytes** (for hashing) keep the full digests, which is the Merkle property;
   - **stored bytes** replace each digest with an 8-byte local id from an interning table (`object(id INTEGER, digest BLOB)`, as Unison's schema does);
   - **in memory**, a 4-byte index into the loaded working set.

   The hash-to-data ratio of the *stored* form then drops to roughly 32 B per definition plus 8 B per reference.
4. **The tag-length-value framing** shared with Moonkale (a u32 tag length plus a u32 payload length per field) costs 8 bytes of headers per field. That is fine for hashing, which never stores it, but wasteful for storage. Storage uses a compact encoding (one-byte tags, varint lengths) that is **converted** to canonical bytes when hashing or verifying. B-06 measures the conversion cost.

## 3. In-memory layout: the "block" the processor sees

A pointer-based tree (a `Box` per node) scatters a definition over the heap: one cache miss per node in the worst case. A **flat layout** keeps a definition contiguous:

```
nodes:    [tag u8 | flags u8 | a u16 | b u32]   (8 bytes per node; a,b = child indices or payload)
extra:    [u32 …]                               (variable-length children, e.g. App args)
literals: [bytes …]                             (strings, big constants)
refs:     [u32 …]                               (indices into the working set's reference table)
```

- 8 bytes per node means **8 nodes per cache line**. A 200-node definition is 1.6 KiB: 25 lines, comfortably in L1.
- Pre-order layout makes the canonical serialisation (also pre-order) a **linear scan**. Hashing becomes streaming over contiguous memory, which is what BLAKE3 is fast at.
- Child indices are relative (u16 where possible), so a definition can be copied or mapped without fix-ups. That suits memory-mapped backends (LMDB, redb) and zero-copy reads.
- This is also the layout CLOSOS's per-thread heap of about cache size would favour ([[Operating Systems and Sophia]] §1).

## 4. Hash width

- **256 bits (BLAKE3 default)**: 128-bit collision resistance, which is enough for any realistic store. A store of 10¹² definitions has a collision probability of about 10⁻⁵³.
- **512 bits** (Unison uses SHA3-512): doubles the key and reference overhead for no practical gain.
- **Truncated display** (the first 8–12 base32 characters, as Unison and git show) for people; full width internally.
- **Local 64-bit ids** in storage (§2.3) are not hashes. They are row ids, with no collision issue.

## 5. Hypotheses and the benchmarks that decide them

| # | Hypothesis | Benchmark |
|---|---|---|
| H1 | Median canonical definition size is 200 B – 4 KiB for sophialang and for ingested Julia/Rust code | B-07 (corpus statistics) |
| H2 | Definition-level hashing costs < 1 µs per definition, so it is irrelevant next to parsing | B-01 (hash throughput by size) |
| H3 | Local-id references shrink the store by ≥ 30 % compared with inline digests | B-05 (store size by encoding) |
| H4 | Subterm sharing with T = 64–256 B gives a dedup gain greater than its overhead only on generated or macro-heavy code | B-07 (dedup ratio by T) |
| H5 | The flat layout hashes and traverses ≥ 3× faster than a boxed tree | B-02 (encode and hash, flat vs boxed) |
| H6 | Compact storage encoding plus conversion beats storing canonical bytes when reads outnumber writes | B-06 |

Results go into [[Benchmark Results]] with the machine, commit and date. A hypothesis that fails is recorded as failed (SR-73).

**First results (2026-10-20, synthetic code):**
- H3 is **refuted** on the synthetic corpus: local ids would save 1.9 %, not ≥ 30 %.
- The framing costs about 40 bytes per term node, not 8–16, so B-06 (a compact storage encoding) is the lever that matters.
- H2 is consistent.
- H1 and H4 need real code.

## Related

[[Benchmark Plan]] · [[Compression and Canonical Form]] · [[Hardware for Sophia]] · [[SPEC-001 Sophia Store v0]]
