#implementation #measurement

> M-1 and M-2 of [[Storage Measurements]] (B-033): how small a SQLite store gets when it keeps only what cannot be rebuilt, whether the lean copy still holds everything a rebuild needs, and what rebuilding costs. Works on copies; the input store is opened read-only. Source: `lean_layout.rs`. Results: [[Lean Layout Measurements]].

```sh
sophia --store STORE.sqlite ingest-code julia LinearAlgebra <stdlib>/LinearAlgebra/src   # or ingest-mm set.mm
cargo run --release -p sophia-ingest --example lean_layout -- STORE.sqlite OUTDIR
```

Needs the `zstd` command on `PATH` (the workspace has no zstd crate). The store must hold exactly one ingest (one manifest): L2 drops the bytes of every record that is not a source or a manifest. The verification refuses a store with more manifests, and fails if a stored record is not rebuilt. The input is opened with `immutable=1`, so nothing is written beside it, and it may not lie in OUTDIR.

`lean_layout --verify LEAN.sqlite L0.sqlite` runs only the verification. It is used to show that damaged copies fail.

**The ladder.** Each layout is a copy of the one before, transformed with SQL, then `VACUUM`ed; the file size and SQLite's `dbstat` per table are printed.

| Step | What it does |
|---|---|
| L0 | `VACUUM INTO` of the input: the baseline, without free pages |
| L1 | empties the `refs` and `nominal_refs` blobs; `t_ref` and `t_nominal_ref` keep the edges (SP-011 option 2) |
| L2 | empties canonical bytes and display of every record except sources and the manifest |
| L3 | a new record table `t_def` keyed by an integer; edges as integer pairs (`t_edge`, with a reverse index); language, normaliser, kind and status by a dictionary `t_dict`; the component digest only for multi-member components |
| L4 | the ingest's log replaced by one entry naming the manifest; each source text compressed with `zstd -19` |
| L5 | the floor: names (except those of the sources and manifest), lineage, annotations, method-table relations and facts deleted. Nominal entities stay, because records' nominal references bind to them. Queries by name need `t_name`, so this is a bound, not a usable layout |

**The alternative to L2.** `compressed_bytes` takes what L2 drops and compresses it with `zstd -19` in blocks of 256 records in insertion order, and as one stream (`--long=27`). It does not build that layout. It gives the size the bytes would take if they were kept.

**Verification (L4 and L5).** From the lean file, with L0 for what is not rebuilt:
- decompress the sources and check that each still hashes to its id;
- decode the manifest and rebuild every record with the frontend, the same path as `source::reconstruct`. The source records and the manifest record are rebuilt too: `source_record` and `Manifest::record`;
- check that the rebuilt ids and the stored ones are the same set. The only exceptions are stub rows that L3 adds for edge targets that are not records; their number is printed;
- check that for each record the dictionary gives back language, normaliser, kind and status, and that index, component size, the component (for multi-member components) and the outgoing edges equal the rebuilt record's;
- check that the rebuilt canonical bytes and display equal L0's for every record whose payload L2 drops;
- for records whose normaliser is not Moonkale's, check that the rebuilt bytes give the id (`Record::verify` per component). Moonkale's ids come from the crate itself, so for those the id equality is the check.

`nominal_refs` are not rebuilt: they bind to nominal entities, which are store state, as in `source::first_difference`. The lean store's nominal edges are compared with L0's `t_nominal_ref` instead.

Damaged copies of LinearAlgebra's L5 (or L0) fail, each checked on 2026-11-20:
- one manifest-to-source edge deleted (`refs` differ);
- one display changed in L0 (a payload not reproduced);
- one nominal edge deleted;
- a second manifest added (refused).

**M-2 timings.**
- **Julia:** the medians of 5 runs of:
  - the whole `module::ingest`;
  - parsing every file;
  - `julia::module` (resolution and ids).

  Per file: the median parse time. Per identified definition: the median time of `definition` (its WP-66 parts) given the references the resolution found. The figure "with a reference sidecar" adds the parse time of the definition's file. It estimates a rebuild that had each definition's resolved references stored beside it, which nothing implements. It leaves out reading the sidecar and `to_record` (references replaced by ids, hashing).

  Each timed call's result is checked after the clock stops, and is dropped outside the timed span.
- **Metamath:** the median of 3 runs of `metamath::parse` on the whole file.

**The block compression figure** leaves out the framing and offsets a block needs to give back one record.

**What it does not measure.** Query speed on the lean layouts: the views and the store code read the old tables. Integer keys for the other id-keyed tables (lineage, annotation subjects, name targets, relation members) are not measured. Neither is the cost of rebuilding inside the store's write path.
