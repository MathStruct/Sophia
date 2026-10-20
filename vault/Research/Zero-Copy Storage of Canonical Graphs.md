#comparison #design

> Daniel asked (SR-101) whether Sophia's canonical graph can map almost directly onto LMDB values, so that storing a sophialang program needs no serialisation layer. **Answer: it already does, apart from one thin record wrapper, and reading in place works today for the framing. But the measurement (§3) moves the question.** An LMDB record read costs about 0.2 µs borrowed, and 0.87 µs with the copy and record decode that the store does today. Decoding the core term into heap objects costs **11.6 µs, 13× the whole store read**. Zero-copy *storage* would save about 0.5 µs per record. A *flat, in-place-readable term layout* is where the order of magnitude is, and it matters to the interpreter, compiler and printer, not to the store or the query executor. A zero-copy representation stays an optimisation: correctness never depends on it.
>
> Sources: Chu, *MDB: A Memory-Mapped Database*, 2011 (LMDB); the `heed` crate (borrowed `&[u8]` values); Cap'n Proto, FlatBuffers and `rkyv` (in-place readable formats); [[Block Size and Hash Overhead]]; [[Compression and Canonical Form]]. code: `Lmdb::with_record_bytes`; `sophia-bench-report --zero-copy`.

## 1. What is stored today

The write path:

```text
sophialang Definition ──normalise──▶ canonical bytes (WP-66 tag-length-value)
                                       │  (this is what the id hashes)
Record { id, lang, kind, component, index, bytes, display, refs, … }
        ──codec (same framing)──▶ one LMDB / redb value, keyed by the 32-byte id
```

- **No JSON, no SQL text, no source text** is stored as the representation. The canonical bytes *are* the program, and the store keeps them verbatim.
- The record wrapper adds the fields the store indexes (kind, component, references) and the display data (names). It uses the same framing.
- SQLite and DuckDB store the same canonical bytes in a BLOB column, with the indexed fields as real columns.

So in the sense the review meant (no intermediate representation), Sophia is already there. What remains is **reading** without building heap objects.

## 2. What "zero-copy" can mean here

LMDB maps the file into memory, and `heed` hands out `&[u8]` slices that point into the map, valid for the read transaction. Four levels:

| Level | What happens | Allocation |
|---|---|---|
| borrowed | look up the id and get a slice into the map | none |
| copied | copy the slice into a `Vec` (what `kv.rs` does today, because `ReadTx` returns owned values) | one |
| walked in place | read the tag-length-value framing directly from the slice | none |
| decoded | build a `Record`, then a sophialang `Definition` with boxed `Term`s | many |

The framing can be walked in place: `canonical::Reader` borrows. References to other definitions are 32-byte digests, so following one is another B-tree lookup, not a pointer dereference. References inside a recursive group are indices.

## 3. Measurements

The run (2026-10-20):
- LMDB, 10⁵ synthetic definitions, mean canonical size 3816 B;
- 1000 random probes in one read transaction, median of 5 runs;
- AMD Ryzen 7 7800X3D;
- `sophia-bench-report --zero-copy --query-size 100000`.

| Step | ns per record | Added by this step |
|---|---|---|
| get, borrowed from the map (no copy) | 207 | the B-tree lookup and page touch |
| get + copy into a `Vec` | 620 | +413: allocation and a 3.8 KB memcpy |
| get + walk the framing in place (no allocation) | 323 | +116: reading every field once |
| `ReadTx::record` (copy + decode the record) | 868 | +248 over the copy: the record's own fields and `Vec`s |
| record + decode the core term (heap objects) | 11 597 | +10 729: boxed `Term`s, strings, vectors |

**What this says:**
- **The store read is cheap, and copying is most of it.** Borrowing would save about 0.5 µs of 0.87 µs. That is worth having for the query executor's `refs_of`, `kind_of` and `uses` lookups, which today decode a whole record to read one field.
- **Building heap terms is the real cost:** 13× the store read. A program that touches many definitions (the interpreter walking code, the printer, a compiler pass) pays this, not the database. That is the case for a flat term layout (B-02) and in-place traversal.
- **Mapping the canonical graph onto LMDB values is not where time is lost.** The representation is already direct.

## 4. The obstacles, and how Sophia's design meets them

| Obstacle | Here |
|---|---|
| **Lifetimes:** a borrowed slice is valid only inside its read transaction | `Backend::read` already takes a closure; a borrowing API is closure-shaped too (`with_record_bytes(ids, \|bytes\| …)`) |
| **Alignment:** in-place formats like Cap'n Proto need aligned words | TLV is byte-oriented, so it needs no alignment. The price is sequential access: no O(1) jump to the k-th child |
| **Variable-length fields** | length-prefixed, so they are walked rather than indexed |
| **Format evolution** | the framing is versioned (`store-v0`, `query-v0`). A new physical format would be a migration of the *physical* encoding, never of identity (SPEC-006 §6, SR-88) |
| **Pointers between nodes** | content ids: lookups, not pointers. A flat in-component arena could use offsets |

## 5. Recommendation

1. **Keep the canonical bytes as the identity and the stored form.** They are stable, hashed and verifiable.
2. **Add borrowing access paths where profiling shows decode dominates:**
   - the executor's `refs_of`, `kind_of` and `uses` need only fields of the record, not the term;
   - a borrowed field reader avoids building the whole `Record`.
   
   This is a `ReadTx` extension with closures, implemented per backend (LMDB and redb can borrow; SQLite and DuckDB copy).
3. **A flat physical layout** (B-02: an arena with child offsets, like `rkyv` or FlatBuffers) is worth it only if term traversal in place becomes hot: the interpreter (M4) and the compiler (M5). It would be stored *beside* the canonical bytes as a derived representation, or replace them physically with the canonical bytes recomputed on demand for hashing. Either way identity is unchanged.
4. **Never require zero-copy for correctness** (SR-101). The copying path stays the reference and the test oracle.
