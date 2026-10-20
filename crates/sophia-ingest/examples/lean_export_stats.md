#implementation #measurement

> Measures a lean4export NDJSON file (format 3.1) before Sophia reads it: declarations by kind, the shared expression graph, and the parts of it reachable from statements, from definition values and from proofs only. Source: `lean_export_stats.rs`. Results: [[Lean Overview]].
>
> Sources: lean4export's `format_ndjson.md` (3.1.0). code: `lean_export_stats.rs`

```sh
cargo run --release -p sophia-ingest --example lean_export_stats -- $SOPHIA_CORPORA/mathlib.ndjson
```

- No JSON library: each line is one object, and only a few integer fields are needed (`ie`, children by key, `type`, `value`, `rhs`), so a line's kind is its first key other than `ie`, and integers are read after their keys.
- Children per node in one array indexed by `ie` (12 bytes per node: 1.3 GB for Mathlib), then three marking passes from the roots.
- "Repeated statements" are theorems whose `type` is the same expression node as another theorem's: equal including binder names.
