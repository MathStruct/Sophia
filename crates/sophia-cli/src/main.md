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
