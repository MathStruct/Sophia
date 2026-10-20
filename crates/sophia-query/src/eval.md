#implementation

> The native executor: semi-naive bottom-up evaluation over any backend's `ReadTx`. Source: `eval.rs`.
>
> Sources: Ullman, *Principles of Database and Knowledge-Base Systems* (semi-naive evaluation); [[SPEC-007 Sophia Query Language]] §6. code: `eval.rs`

- Strata run in dependency order. In a recursive stratum, each round evaluates every rule once per recursive literal, with that literal reading only the previous round's new tuples (the delta).
- Base literals read through access paths (`kind_of`, `records_of_kind`, `refs_of`, `dependants`, `name`, `names_of`, `relation`, `nominal`, `annotations_of`, `facts_of`), memoised per run, or through a full scan (`all_kinds`, `all_refs`, …) when no column is bound.
- Local relations keep hash indexes on the column sets the plans look them up by.
- `Limits` cap join steps, rows and **derived tuples**. The derived-tuple cap is the memory bound. The step budget alone is not one: a closure of 10⁵ definitions grew past 14 GB before reaching 50M steps. Each round's new tuples are deduplicated against everything known as they are produced, so duplicates never pile up. Without counts, the select stops one row after the row cap and marks the result truncated.
- `Stats` count lookups, scans, derived tuples and rounds (reported by the CLI and the benchmarks).
- `count` with no grouping columns over no solutions gives one row of zeros, as SQL does.
