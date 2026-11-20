#implementation #julia

> **Prints E-3's scores for runs of `dispatch_facts.jl`:** per run (one added method or a few), the number of call facts, the calls that changed, the two rules' scores against them, the recorded instances, Julia's invalidations and the two rules' scores against those. Results: [[Dispatch Invalidation]].
>
> Sources: code: `julia_invalidation.rs`, `src/invalidation.rs` ([[invalidation]]), `tests/julia-semantics/dispatch_facts.jl` ([[dispatch_facts]])

```sh
cargo run --release -p sophia-ingest --example julia_invalidation -- $SOPHIA_CORPORA/e3/*.tsv
cargo run --release -p sophia-ingest --example julia_invalidation -- --misses run.tsv   # also lists every miss
```

Scores are `tp/fp/fn`, then precision and recall. `changed~julia` scores the instances that rely on a call that truly changed against Julia's log: how far Julia itself invalidates beyond what changed. `unrecorded` counts instances Julia's log names that the run did not record (outside the recorded closure); they are not in the instance scores, so a non-zero value limits the recall figure.
