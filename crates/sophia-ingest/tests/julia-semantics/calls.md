#implementation #julia

> The ground truth for parts 3 and 4 of [[Julia Conformance]] (dispatch, world change): Julia lists, for every method a package defines, the global bindings its lowered body uses, and what each binding is (a function with its methods, a type, a module). Source: `calls.jl`.
>
> Sources: Julia ≥ 1.12 (`Base.visit(Core.methodtable)`, `Base.uncompressed_ast`). code: `calls.jl`, `src/conformance.rs`, `examples/julia_dispatch.rs`

```sh
julia crates/sophia-ingest/tests/julia-semantics/calls.jl LinearAlgebra > calls.tsv
julia crates/sophia-ingest/tests/julia-semantics/calls.jl Disp crates/sophia-ingest/tests/julia-semantics/dispatch/src | sort > dispatch/truth.tsv
# part 4, from crates/sophia-ingest/tests/julia-semantics: the table before, and after each change
julia calls.jl World world/src | sort > world/truth/base.tsv
for c in world/src/changes/*.jl world/outside/*.jl; do julia calls.jl World world/src $c | sort > world/truth/$(basename $c .jl).tsv; done
```

- One line per (method, binding used): file, line, the name, the binding (`Module.name` where Julia looks it up), its kind (`function`, `type`, `module`, `other`, `undefined`), its owner (`parentmodule(f).nameof(f)` for a function), `call` or `value`, how many methods the function or type has in all of Julia, and which of them are in this package (`file:line`).
- The body is the **lowered** code: macros are expanded, a closure is a method of its own at its own line, and a qualified `A.h(x)` lowers to `getproperty(A, :h)`, which the script reads as the binding `A.h`.
- Only methods whose file is under the package's `src/` (or under SRCDIR when given) are printed. Names print without `Main.` so a package loaded by `include` reads like one loaded by `using`.
- **A change** (part 4) is a third argument: a file evaluated after the package is loaded, into the package's module if it lies under SRCDIR (its methods are then the package's and are listed), otherwise into `Main` (another package's code: its methods count in `nmethods` but are not "here"). The table is printed in the world after the change, so a binding the change shadows (a local `sum` where Base's was used) prints as the new one.
- A method that another one replaced (same signature, defined later, e.g. by another package) stays in Julia's method table but is skipped: only methods `m` with `which(m.sig) === m` are printed (an ambiguity keeps the method).
- Each run takes about a minute (it visits every method in Julia's method table).
- The output is in method-table order; sort it before committing it (the fixture's `truth.tsv` is sorted).

This is Julia, not Sophia code: it measures Julia. Sophia stays Rust (S-ADR-0004).
