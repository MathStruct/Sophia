#implementation

> Name resolution shared by the parser and the printer, built-ins, and operator precedence. Source: `resolve.rs`.
>
> code: `resolve.rs`

- `Env` is the store as the syntax layer sees it: lookup by absolute path, constructor names, names of a target. It must be `Sync`, because parsing runs on its own thread.
- `candidates(ns, path)` gives the innermost namespace first, then outwards, then the path as written.
- `BUILTINS` are the intrinsic nominal entities (types such as `Int64` and `Any`, operators such as `+` and `==`).
- `spellings(abs)` lists the ways to write a path, shortest suffix first. The printer takes the first one that resolves back.
