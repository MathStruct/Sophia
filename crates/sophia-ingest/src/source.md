#implementation

> Source files as content-addressed records, and the **ingest manifest** that says how records were derived from them, so that every record an ingest stored can be rebuilt from the store alone and checked field by field. The precondition for ever dropping derived bytes ([[Current Inventory]]). Source: `source.rs`.
>
> Sources: [[Storage and Materialization]]; [[SPEC-006 Worlds, Artifacts and Evidence]] §6a (a manifest is a transformation record); [[Review 2026-10-20 Real Code 2 (external)]]. code: `source.rs`, `tests/reconstruct.rs`

- **Source record:** language `source`, normaliser `sophia-source@0`, kind `source-file`, bytes = the file exactly. Its id covers the content only, so `Record::verify` (Sophia's own hashing) checks it. The path is display data.
- **Manifest record:** language `sophia-ingest`, kind `ingest-manifest`. Its bytes (canonical framing) hold the frontend and its version (`sophia-ingest/julia-module@0`, `sophia-ingest/metamath@0`), the normaliser whose ids the records carry (`moonkale-wp66@7`, `sophia-metamath@2`), the package, and per file in reading order its relative path, source id and include module path, plus dynamic includes. Its `refs` are the source records, so a manifest keeps its sources reachable. Named `ingest.jl.<pkg>` / `ingest.mm.<db>`; a changed manifest moves the name (`Store::update`).
- **`reconstruct(store, manifest)`:** reads the manifest and sources, runs the frontend compiled into this binary, and compares every rebuilt record with the stored one: id, language, normaliser, kind, component, index, component size, bytes, display, status, refs. `nominal_refs` are not compared: they bind to nominal entities, which are store state. A manifest naming another normaliser than this binary's is refused, not guessed.
- **Not covered yet:** per-file Julia, Rust and Python ingest (`store_code`) store no manifest; the environment beyond versions (Julia's Base bindings, SP-013) is not in the manifest.

Tested on all four backends (`tests/reconstruct.rs`): Julia fixtures and Metamath rebuild identically; every stored Julia record is among the rebuilt ones; a manifest with a changed source is detected; re-ingesting writes nothing. At scale: `sophia reconstruct ingest.jl.LinearAlgebra` and `ingest.mm.set` ([[Current Inventory]]).
