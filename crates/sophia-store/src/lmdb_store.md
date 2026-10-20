#implementation

> The LMDB backend, via `heed`: a memory-mapped B+tree with one writer and zero-copy reads. Source: `lmdb_store.rs`.
>
> Sources: Chu, *MDB*, 2011; heed 0.22; [[Database Comparison]]. code: `lmdb_store.rs`, [[kv]]

## Shape

A directory holds `data.mdb`. `Lmdb::open` reserves 64 GiB of address space (`open_with_map_size` changes that); no disk is used until data is written. There is one database per [[kv]] table. Read transactions are thread-local (`WithTls`), which suits the store, since every read runs inside one closure on one thread.

## Findings (2026-10-20)

- **The fastest backend** on the synthetic corpus: 17 600 definitions/s ingest and 0.9 µs record lookups at 10⁵ ([[Benchmark Results]]).
- **LMDB rejects zero-length keys, also as the start of a range.** A scan from the empty prefix failed with `MDB_BAD_VALSIZE`, so the scan now starts unbounded. The conformance suite now covers this on every backend.
- **Keys are limited to 511 bytes**, so name paths and meta keys must stay below about 470 bytes. Moonkale's paths are far shorter, but the limit is not yet enforced with a clear error.
- **A fixed map size:** a store that outgrows it needs reopening with a larger one.
- **One writer at a time**, as with every backend here (all use one connection or one write transaction).

**`with_record_bytes`** (2026-10-20): hands a closure each record's stored bytes, borrowed from the memory map inside one read transaction, with no copy and no decode. It exists for the zero-copy measurements ([[Zero-Copy Storage of Canonical Graphs]]) and is the prototype of a borrowing access path.
