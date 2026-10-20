#implementation

> Canonical form, canonical bytes, and identities of queries. Source: `canon.rs`.
>
> Sources: [[SPEC-007 Sophia Query Language]] §4; [[SPEC-001 Sophia Store v0]] §4 (the shared identity rule). code: `canon.rs`

- `canonicalise` numbers variables by first occurrence, drops unreachable predicates, numbers predicates by first visit from the select, and sorts each predicate's rules by their bytes (dropping duplicates). It repeats until stable (at most 8 rounds) and keeps the smallest form seen. Literal order is not normalised ([[SP-009 Canonical form of queries]]).
- `encode`/`decode`: WP-66 tag-length-value framing, as for sophialang terms.
- `query_id`: the member id of a one-member component of the language `sophia-query`, normaliser `sophia-query@0`, so a stored query verifies with `Record::verify` like any definition.
- `result_id`: H(query id, snapshot, parameters).
- `record`: the store record; id constants the store knows become `refs` or `nominal_refs`.
