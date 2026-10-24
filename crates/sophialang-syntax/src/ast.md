#implementation

> The surface tree, before names are resolved. Source: `ast.rs`.
>
> code: `ast.rs`

`SExpr` contains paths, literals, calls, infix operators, lambdas, Π-types, `let`, `match`, and binder forms (`(x::T)`, `{x}`, `@erased x`) that are only valid before `->` or `-->`. `SItem` is one top-level definition, written in one of the surface forms (`function` sugar, typed method, `const`, `data`, `layout`, `relation`, `rule`, `test`, `postulate`).

**`SExpr`** (2026-10-24) implements `Clone`, `PartialEq`, `Debug` and `Drop` by hand, each going through `deep::grow` once per level, so a syntax tree of any depth can be copied, compared, printed and dropped (SP-015). `SKind` keeps its derives; recursion passes through `SExpr` at every level.
