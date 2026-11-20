#review #external

> **Codex's review of B-033 (M-1, M-2; `lean_layout.rs`), 2026-11-20, defects only.** Eleven findings. All are right. Eight led to a fix in the program, two to a statement in the notes, and one (7) was left as it is. Most concerned how much the verification proves, and it is now stronger: the manifest and sources are checked like other records, the dropped payloads are compared with L0's, and the nominal edges too. After the fixes the sizes are unchanged and both corpora still verify. Four damaged copies are each refused. The M-2 timings moved by up to 4 %.
>
> Sources: `codex exec -s read-only` with a brief naming the requirement and the files, no reasoning sent (`meta/scratchpad/review-brief-B-033.md`); its answer is `meta/scratchpad/REVIEW-from-codex-B-033.md`. code: `crates/sophia-ingest/examples/lean_layout.rs`, [[lean_layout]]; results in [[Lean Layout Measurements]].

| # | finding | right? | what was done |
|---|---|---|---|
| 1 | an input inside OUTDIR (e.g. `out/L0.sqlite`) is deleted before it is read | **right** | refused: the input may not lie in OUTDIR |
| 2 | the read-only open of a WAL-mode store can create `-shm` beside the input | **right**, seen: the first runs left `la.sqlite-shm` and `mm.sqlite-shm` (12:34), the input's main file untouched | the input is opened with `immutable=1`; the rerun wrote nothing beside it |
| 3 | the manifest's id and the manifest-to-source edges were not checked; source metadata skipped | **right** | the manifest and source records are rebuilt (`Manifest::record`, `source_record`) and compared like every other record, edges included. A copy with one manifest edge deleted now fails |
| 4 | the one-manifest assumption is not checked | **right** | a store with more than one manifest is refused (checked with a copy with a second manifest) |
| 5 | an edge to a stub row and the nominal edges were not verified | **right** | edges are compared exactly with the rebuilt references, the manifest's included, so an extra edge to a stub fails; the number of stubs is printed (0 in both corpora). Nominal edges are compared with L0's `t_nominal_ref`; a copy with one deleted fails |
| 6 | the dropped bytes and display were never compared with what was dropped (display is not hashed) | **right** | the rebuilt bytes and display are compared with L0's for every dropped payload; a copy of L0 with one display changed fails. All 3 284 and 98 136 match |
| 7 | L5 keeps nominal entities nobody references, and the sources' and manifest's display | right, small | not changed: nominal entities are store state (records bind to them). LinearAlgebra's `t_nominal` is 72 KiB, set.mm's 4 KiB. Stated in [[lean_layout]] |
| 8 | the block compression figure loses record boundaries: no framing or offsets | **right** | stated in both notes with an estimate of the missing part (4–8 B per record, under 0.8 MiB for set.mm). Not measured |
| 9 | the sidecar estimate leaves out reading the sidecar and `to_record` | **right** | stated in both notes; the figure was already called an estimate |
| 10 | timed calls are dropped inside the timed span; parse and encode errors were discarded | **right** | each result is checked after the clock stops and dropped after it. No call failed in the rerun, and the timings moved by up to 4 % |
| 11 | empty inputs divide by zero or index `texts[0]` | right, not reachable on the corpora | guarded: an error instead of a panic |
