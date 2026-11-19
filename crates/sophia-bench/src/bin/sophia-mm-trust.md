#implementation #experiment #benchmark

> E-0 (B-021): proof trust records end to end on a real Metamath library. Checks every proof with metamath.exe, records one annotation per proof ([[trust]]), and times the query "which theorems rest only on checked proofs". Prints the Markdown that goes into [[Benchmark Results]]. Source: `sophia-mm-trust.rs`.
>
> Sources: [[Semantic Boundary Experiments]] E-0; [[SPEC-006 Worlds, Artifacts and Evidence]] §5; metamath-exe (github.com/metamath/metamath-exe). code: `sophia-mm-trust.rs`

```sh
cargo build --release -p sophia-bench --bin sophia-mm-trust
./target/release/sophia-mm-trust --exe $SOPHIA_CORPORA/metamath-exe/metamath $SOPHIA_CORPORA/set.mm/set.mm --db set --runs 5
```

- **Store:** SQLite in a temporary folder under `--dir` (default `$SOPHIA_CORPORA`, never `/tmp`), so its size on disk means something; filled by `store_metamath_source`, whose source manifest id becomes each annotation's `inputs`.
- **Check:** one metamath.exe session, `verify proof *`, timed with reading the file. Its version comes from its banner.
- **Simulated failure first:** `--taint LABEL` (default `a1i`) is marked failed along with the real failures, and the two queries count how far one bad proof reaches. Then the real verdicts are added (a re-check: `a1i` gets a `checked` beside its `check-failed`) and the queries are timed, median of `--runs`.
- **Sizes:** the proof records' content bytes; the annotations' logical bytes (payload, ids, kind, status: an estimate, not the backend's encoding); the SQLite file plus WAL before and after.
- **E-0 fails if** the annotations cost more than the proofs they annotate, or the trusted closure is not expressible: compare the two size rows, and see [[trust]] "Limits" for what the query cannot say.

## Results

See [[Benchmark Results]] (E-0).
