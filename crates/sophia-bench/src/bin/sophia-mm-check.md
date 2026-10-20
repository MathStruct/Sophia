#implementation #experiment

> Checks Sophia's Metamath proof-dependency answers against `metamath.exe`, the reference implementation, which shares no code with Sophia. The acceptance test for B-16 rung 1 that the external review asked for. Source: `sophia-mm-check.rs`. Results: [[Metamath Overview]] and [[Real Code Corpora]] §4.
>
> Sources: [[Review 2026-10-20 Real Code (external)]]; metamath-exe (github.com/metamath/metamath-exe, built from source with `gcc -O2 src/*.c`). code: `sophia-mm-check.rs`

```sh
sophia-mm-check --exe PATH/metamath set.mm [--db set] [--sample N] [--seed S] [LABEL ...]
```

- Ingests the database into a temporary LMDB store.
- **Sophia:** the query the benchmark times, the closure of a proof's references through proofs, filtered to the library's `axioms` and `definitions` collections.
- **metamath.exe:** `show trace_back LABEL /axioms /essential` for every label in one session (commands on stdin, scrolling off).
- **Comparison by statement id:** two labels with identical statements are one id in Sophia. metamath.exe's list is split into axioms and definitions by the same rule as Sophia's (`df-`), and anything else it lists is reported.
- The sample is the named labels plus a seeded draw over all theorems.
