#implementation

> Canonical form, canonical bytes, and identities of queries. Source: `canon.rs`.
>
> Sources: [[SPEC-007 Sophia Query Language]] §4; [[SPEC-001 Sophia Store v0]] §4 (the shared identity rule). code: `canon.rs`

- `canonicalise` is **exact**:
  - it numbers variables by first occurrence and drops exact duplicate rules;
  - the result is the smallest encoding over every valid traversal from the select: predicates by first mention, each predicate's rules sorted with not-yet-numbered predicates masked, and every order of tied rules tried;
  - beyond `CANON_BUDGET` (4096) traversals the query is refused, never given an unstable id;
  - literal order is not normalised ([[SP-009 Canonical form of queries]]).
  
  `tests/properties.rs` checks invariance under random permutations, duplicates, unreachable predicates and renaming.
- `encode`/`decode`: WP-66 tag-length-value framing, as for sophialang terms.
- `query_id`: the member id of a one-member component of the language `sophia-query`, normaliser `sophia-query@0`, so a stored query verifies with `Record::verify` like any definition.
- `result_id`: H(query id, snapshot, parameters).
- `record`: the store record; id constants the store knows become `refs` or `nominal_refs`.
