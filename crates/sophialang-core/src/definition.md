#implementation

> sophialang definitions: kinds, declared types, bodies. Source: `definition.rs`.
>
> Sources: [[SPEC-003 Sophialang Core v0]] §1. code: `definition.rs`

- **Content-addressed kinds:** `method`, `constant`, `data`, `layout`, `relation`, `rule`, `test`, `postulate`.
- **Nominal kinds:** `generic`, `struct`, `abstract`, `primitive` (`NominalKind`). They are not definitions: the store creates them with fresh ids (SPEC-001 §4.3).
- **Bodies:** none (a postulate or relation), a term, constructors (`data`), fields (`layout`; field names are hashed because `x.r` observes them), or a rule (logic variables, head, premises, an optional declared weight).

`is_incomplete` is true if any hole or unresolved name occurs.
