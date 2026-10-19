#implementation

> The surface tree, before names are resolved. Source: `ast.rs`.
>
> code: `ast.rs`

`SExpr` contains paths, literals, calls, infix operators, lambdas, Π-types, `let`, `match`, and binder forms (`(x::T)`, `{x}`, `@erased x`) that are only valid before `->` or `-->`. `SItem` is one top-level definition, written in one of the surface forms (`function` sugar, typed method, `const`, `data`, `layout`, `relation`, `rule`, `test`, `postulate`).
