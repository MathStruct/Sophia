Review a new measurement program for defects only. Do not edit any file; read-only.

File: crates/sophia-ingest/examples/lean_layout.rs (new, untracked) in this repository (Rust workspace "Sophia", a content-addressed code store).

Requirement it implements (vault/Storage and Materialization/Storage Measurements.md, M-1 and M-2):
- M-1: how much smaller is a SQLite store that keeps only authority (source texts and the ingest manifest), ids, kinds and one copy of the reference edges? Method: build lean copies of an existing store (schema in crates/sophia-store/src/sqlite.rs, SCHEMA), measure size, and show the lean copy loses nothing: rebuild every record from the lean copy's own sources with the frontend (crates/sophia-ingest/src/source.rs `reconstruct` does this against the full store) and check that ids, language, normaliser, kind, status, component (for multi-member components), index/size and reference edges are all still held. The input store must not be modified.
- M-2: what does rebuilding canonical bytes cost per definition and per file (Julia module ingest: crates/sophia-ingest/src/module.rs `ingest`; Metamath: crates/sophia-ingest/src/metamath.rs `parse`)?

Look for: wrong numbers (a size or time that does not measure what its label says), a verification that could pass while data was actually lost (e.g. a check that compares something with itself, a filter that silently excludes rows, edges whose targets are not records), SQL that misses rows or duplicates them, an input store that could be written, panics on plausible input, timing that measures the wrong thing (dead code elimination, cached trees, including setup).

Answer: a numbered list of defects, each with the line number, what goes wrong, and a concrete scenario. Mark which you verified (e.g. by reading the called code) and which are guesses. No praise, no restating the code. If you find nothing, say so.
