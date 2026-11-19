#implementation #julia

> CI checks part 9 of [[Julia Conformance]] (include order) against committed Julia truth, without invoking Julia. Source: `includes.rs`.
>
> Sources: code: `includes.rs`, `julia-semantics/includes/truth.tsv`, `julia-semantics/includes.jl` ([[includes]]), `src/conformance.rs` (`include_ledger`), `src/module.rs` (`discover`)

```sh
cargo test -p sophia-ingest --test includes
```

- `include_ledger_on_the_fixture`: every file and method of the fixture `Inc` with its expected outcome, and the one reordered file (`late.jl`).
- `a_missed_file_makes_a_content_reference`: a method in a file Sophia does not read leaves the other method alone, so a caller refers to it by content.
- `a_missing_file_in_a_dead_branch_refuses_the_package`: discovery fails on an include of a file that does not exist, in a temporary package.
- `falsified_truth_gives_each_outcome`: truth changed so that a file runs in another module, a file runs twice, an extra file runs after all, and the order changes; each is found.
- `malformed_truth_is_counted`.
