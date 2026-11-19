#implementation #julia

> The ground truth for part 6 of [[Julia Conformance]] (closures and captured variables): for every closure a package's methods create, Julia's own account of each name in it, as lowering left it: an argument or local of the closure, a captured variable (a field of the closure type, boxed or not, or a static parameter of the enclosing method), or a global binding. Source: `closures.jl`.
>
> Sources: Julia ≥ 1.12 (`Base.visit(Core.methodtable)`, `Base.uncompressed_ast`, `fieldnames` of a closure type, `CodeInfo.slotnames`). code: `closures.jl`, `src/conformance.rs` (`closure_ledger`), `tests/closures.rs`, `examples/julia_closures.rs`

```sh
# from crates/sophia-ingest/tests/julia-semantics
julia closures.jl Clo closures/src | sort > closures/truth.tsv
julia closures.jl LinearAlgebra <stdlib>/LinearAlgebra/src > closures-la.tsv   # about 20 s
```

- One line per closure method: file, line, the closure type's name, `own` or `creator`, its arguments, its captured variables, its other locals, its globals. Lists are joined by `,`, `-` when empty.
- **Which methods are closures.** A method whose first signature parameter is a type named `#…`, whose name is bound to the type itself, and which is not a named function's type: since Julia 1.12 `typeof(solo)` is bound as `#solo` too, so a type whose instance is bound under the function's own name (`solo`) is left out. Keyword bodies (`#f#3`) are functions, not closure types, and are left out the same way.
- **Captured** is `fieldnames` of the closure type: `name`, or `name:box` when the field's type is `Core.Box` (the variable is assigned after it is captured: `c += 1` inside the closure, or a local assigned after the closure is made). A static parameter the closure uses (`sp(x::T) where {T} = () -> T`) is a type parameter of the closure type, not a field: `T:sparam`. Fields Julia makes for itself (`#20#21`, the keyword body a closure with keyword arguments holds) are left out.
- **Arguments and locals** are the closure method's `slotnames`: `#self#`, unnamed and compiler-made (`#…`) slots left out. A destructured argument (`((a, b), c) -> …`) is unnamed; its names are locals. A captured variable may also have a slot (Julia unboxes into one); the comparison counts it as captured.
- **Globals** are the `GlobalRef`s of the lowered body (`name=Module.name`), operators and `Core` intrinsics included; only names that appear as identifiers in Sophia's parse are compared.
- **Placement.** A closure's method carries the closure's line (for a `do` block, the line of its body). A generator's or comprehension's body is filed at `none:0`: it is printed under the method that creates it, with `creator`, and matched by its iteration variables.
- Only methods under the package's `src/` (or SRCDIR) are read, and only those `which` still dispatches to.
- A closure with keyword arguments makes several closure types at one line (the positional one and its keyword body); the comparison merges the lines that match one closure of Sophia's.

This is Julia, not Sophia code: it measures Julia. Sophia stays Rust (S-ADR-0004).
