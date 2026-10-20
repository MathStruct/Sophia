#implementation

> Julia modules through `moonkale-content-id`'s module-level resolver (normaliser v5; [[SPEC-008 Module-Level Julia Resolution]]). Source: `module.rs`. The prototype that was here was ported into the crate from Sophia `eb549c6`.
>
> Sources: [[SPEC-008 Module-Level Julia Resolution]]; Moonkale `packages/services/content-id/src/julia.rs` (`6592867c`). code: `module.rs`, `tests/module.rs` (against `golden-moonkale/module.tsv`), `examples/generic_calls.rs`

- `package_root(dir)`: `src/<name>.jl` from `Project.toml` (the caller's job, SPEC-008 §4).
- `discover(root)`: reads the files in include order with the crate's `includes`; literal includes only, others recorded as dynamic; bounded by the crate's constants.
- `ingest`:
  - calls `julia::module`, which resolves and identifies the whole module under one module budget;
  - rebuilds each identified definition's canonical parts from the crate's references, for the record bytes (the same input gives the same encoding);
  - builds records with Moonkale's ids.
- **`uses_generic`:** the crate's `calls_generic`, minus names the enclosing definition binds itself (parameters, locals, `where` type parameters). It becomes records' `nominal_refs` (the store's `uses(d, g)`), which do not affect ids.
