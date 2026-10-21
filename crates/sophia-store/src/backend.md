#implementation

> The interface every storage backend implements. Source: `backend.rs`.
>
> Sources: [[SPEC-001 Sophia Store v0]] §12. code: `backend.rs`

`Backend::read` and `Backend::write` run a closure inside a transaction: `write` commits on `Ok` and rolls back on `Err`. The closure receives `&dyn ReadTx` or `&mut dyn WriteTx`, both object-safe sets of typed operations:
- records, components, dependants;
- names;
- the log;
- nominal entities and relations;
- lineages;
- annotations, facts;
- meta.

**Why typed operations rather than a raw key-value interface:** SQLite can then keep real tables, which the read-only `sql` views need, while a key-value store keeps byte tables. The rules live once, in [[logic]].

**Why closures:** a transaction's lifetime is tied to the backend's own transaction type. Closures keep that out of the public types, which Moonkale's adapter needs to be free of driver types (SPEC-004).

**Access paths for the query executor** (2026-10-20, [[SPEC-007 Sophia Query Language]] §2): `kind_of`, `refs_of`, `all_kinds`, `all_refs` and `nominal_ids` have default implementations built on the other reads, so every backend answers Sophia's queries. A backend overrides one when an index answers it without decoding records (SQLite, DuckDB, and the key-value backends for the two scans).

**More access paths** (2026-10-20, from Metamath): `annotation(id)` reads an annotation by its id, and `memberships_of(member)` lists the nominal entities whose relation contains a definition (the generic functions a method belongs to; the library collections holding a statement). The key-value backends keep a `relation_by_member` table; SQLite and DuckDB have an index on `t_relation(member)`.

**`names_from(base, from, limit)`** (2026-10-21): the names under `base` (`""` or `ns.`) that sort at or after `from`, as a range scan on the name index. It lets the interface list a namespace's direct children by skipping each sub-namespace (`ns.sub.` … `ns.sub/`, since `/` follows `.`) instead of reading every name below it. SQLite and DuckDB answer it with one lower bound and one upper bound on `path`, the key-value backends with a prefix scan from the cursor; the default filters `names_under`. A query with two lower bounds (`path >= base AND path >= from`) made SQLite seek on the wrong one and rescan the namespace on every batch (10× slower, measured); hence the single bound. `names_under` on SQLite now uses the same range instead of `substr(…)`, which could not use the index.
