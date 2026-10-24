#implementation

> Elaboration: surface items → core definitions. Source: `elab.rs`.
>
> Sources: [[SPEC-003 Sophialang Core v0]]. code: `elab.rs`

Names are resolved in this order:
1. local binders (de Bruijn indices);
2. other items of the same text (`Term::Draft`, so recursive groups written together hash together);
3. the store, from the innermost namespace outwards ([[resolve]]);
4. built-ins;
5. `T.c` constructors (via the data type's constructor names);
6. otherwise `Unresolved`, with a warning.

`function f(params)::R body` becomes a Π chain plus a λ chain. An untyped parameter has type `Any` (Julia's rule, SR-29). Binary operators become calls of the built-in generic functions. `match e :: T` takes its constructor names from `T`.

A `relation` becomes `Item::Relation`. The interface creates the nominal entity (S-ADR-0009).

**An explicit-stack elaborator (2026-10-24).** `Elab::expr` runs steps (`Eval`, `Push` / `Pop` / `Truncate` of scope names, `MatchCases`, `Build`) instead of recursing; a match's constructors are resolved by a `MatchCases` step once its data type is elaborated. The scope is indexed by name (`Scope`), so a variable's de Bruijn index costs O(1) instead of a scan. Same terms as before (20 000 random round trips, the sample, 300 generated definitions saved back unchanged); any depth on every target.
