#implementation

> The `sophia` command. Source: `main.rs`.
>
> Sources: [[Milestones]] M1. code: `main.rs`

```sh
sophia [--store PATH] [--backend sqlite|redb] init | sample | gen N [SEED] | ls [PREFIX] | show NAME|ID
       | deps NAME|ID | log [SINCE] [LIMIT] | verify | stats | sql QUERY
```

- The default store is `.sophia/store.sqlite` (or `.redb`).
- `show` prints the **core form** (Rust `Debug`) until the `julia` profile's printer exists (M2).
- `sql` runs through the read-only guard (SQLite only).
- `verify` rehashes every definition.
- Names and ids are interchangeable arguments, and an unambiguous hex prefix works too.

**Known rough edge:** piping into `head` panics with "Broken pipe" instead of exiting quietly.

**`sophia query Q [ARG..]`** (2026-10-20): Q is query text, `@FILE`, or the name of a stored query. Arguments follow the parameters in order; an `Id` argument may be a name. `--explain` prints the canonical text, the id, the result type and the program the executor runs (with magic sets, in Datalog notation). `--sql` prints the SQL translation. `--plain` runs without magic sets. `--save NAME` stores the query.

**`sophia verify`** (2026-11-08) counts three outcomes: valid, failed, and records from another tool's normal form, for which only the component size is checked ([[logic]]). It fails only on the second.
