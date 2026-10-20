#design #julia

> What a name means in Julia, what Sophia resolves, what it approximates, and what it leaves open. 2026-10-20. Includes the fix for the review's P0: **a generic function is (module, name), never a name alone** (SR-127).
>
> Sources: the Julia manual, *Modules* and *Methods* (binding resolution, `import` vs `using`, extending functions); [[SPEC-008 Module-Level Julia Resolution]]; Moonkale `julia.rs` (v6); `crates/sophia-ingest/src/code.rs` (`julia_scopes`, `locate`, `julia_extends`), `sophia_ingest.rs` (`binding`). Tests: `tests/generics.rs`, `tests/module.rs`; [[Julia Conformance]].

## How Julia resolves a global name

1. Every module has its own global scope. A package is one module (plus submodules) spread over `include`d files, evaluated in order.
2. A name used in module M refers to M's binding of it: one M defines, one M `import`ed, or one made visible by `using`. Every module implicitly does `using Base` (and `using Core`).
3. **A generic function is a binding.** `f(x) = …` in module A creates or extends `A.f`. The same text in module B creates `B.f`, a different function.
4. **Extending another module's function** takes either a qualified head (`Base.show(io, x) = …`) or an explicit `import Base: show` before `show(io, x) = …`. After only `using Base: show`, defining a method is an error.
5. A binding may be an **alias**: `Base.Array` is `Core.Array`, re-exported. The function is the object, not the path.

## What Sophia resolves (L3)

| Question | Who answers | How |
|---|---|---|
| Which definition does a module-level name refer to? | the crate (`julia::module`) | per module: exactly one binding and nothing imported under that name; across `include`d files |
| Is a use a call to a generic function? | the crate (`calls_generic`) | several methods bind the name in the module; scope-aware since v6 |
| **Which generic function does a method belong to?** | Sophia (`binding`) | (module path, name), or the function it extends |
| Which module does a qualified head or an `import` name? | Sophia (`locate`) | below |

**The generic key.** `binding(def)`:
- a method with `extends` set belongs to that function: `Base.show(…)`, `Base.:+(…)`, `A.f(…)`, or `show(…)` after `import Base: show` (or `import Base.show`);
- otherwise to (its module, its name), the module coming from the crate (file's include base plus `module … end` blocks);
- a function named like a type in the same module is that type's constructor (`jl.M.T.new`);
- a qualified head that cannot be placed (`Base.$f(…)` in an `@eval` loop) belongs to nothing, and that is reported, not guessed.

**`locate(m, path)`**, Julia's lookup of a module path written in module m:
1. leading dots: relative (`.X` is m's X, `..X` its parent's);
2. the first segment, innermost module outwards: a declared submodule, or an enclosing module's own name;
3. Base's exported submodules (`Broadcast`, `Iterators`, `Threads`, …, from `names(Base)` in Julia 1.13);
4. otherwise a top-level module (`Base`, `Core`, another package).

## The bug this replaced (2026-10-20)

`store_items` grouped methods by the bare name. So:
- `LinearAlgebra.dot`, `LinearAlgebra.BLAS.dot` were one function; so were `axpy!`, `axpby!`, `gemv!` and `check` across LinearAlgebra, BLAS and LAPACK;
- LinearAlgebra's 123 extensions of Base's functions (`*`, `\`, `show`, `getindex`, …) were stored as LinearAlgebra's own functions, so a query for "methods of Base's `*`" could never find them;
- `Base.show(…)` became a function literally named `` `Base.show` `` inside LinearAlgebra.

Measured against Julia ([[Julia Conformance]]), the old rule gave the right owner for **53.7 %** of LinearAlgebra's methods; the new one for **94.5 %**. The external review predicted the same-name case from reading the code; LinearAlgebra had five real instances, and the import and qualified-head cases were far larger. `tests/generics.rs` now covers unrelated same-named functions in sibling modules, `import M: f` and `import M.f`, relative imports (`import ..A: f`), qualified extensions of Base and of a sibling module, operators (`Base.:+`) and constructors.

## What Sophia approximates (and how much it costs)

Measured on LinearAlgebra against Julia's own method table ([[Julia Conformance]]): 2 777 of 2 939 stored methods (94.5 %) have the owner Julia reports.

| Approximation | Disagreements | Example | Proper fix |
|---|---|---|---|
| **re-exports and aliases** not followed | 86 | `Base.Matrix(…) = …` is a constructor of `Core.Array`; `Base.cbrt` is `Base.Math.cbrt`; `Base.uppercase` is `Base.Unicode.uppercase` | a **binding table** for modules Sophia did not ingest (Base, Core), produced by Julia and stored as a cited environment artifact ([[SP-013 Julia bindings of modules Sophia has not ingested]]) |
| **definitions inside macro arguments** | 11 | `@deprecate cholesky!(A, ::Val{false}; check::Bool = true) …` yields a unit named `check` | the crate should not treat a keyword default inside a macro call as a definition; reported to Moonkale |
| `using` bindings | (included above) | a method for a name brought in by `using` | the binding table, as above |
| lines Julia never compiles | 65 not found in Julia | `@static if Sys.isapple()` blocks; `function f end` declarations (no method) | correct for a *source* store: platform-dependent code exists; its methods are world- and platform-dependent |

## What Sophia leaves unresolved, on purpose

- **Which method a call reaches.** That is dispatch, which depends on argument types and the world ([[Julia Multiple Dispatch and World Age]]).
- **Names created by macros**, `@eval` and `include` of computed paths (`dynamic` includes are recorded; functions in such modules stay open).
- **Runtime method definitions** (`eval` in a function body).

## Entity names in the store

| What | Name |
|---|---|
| a package's function | `jl.LinearAlgebra.dot` |
| a submodule's function | `jl.LinearAlgebra.BLAS.dot` |
| a Base function the package extends | `jl.Base.show`, `` jl.Base.`*` `` |
| a constructor | `jl.LinearAlgebra.Diagonal.new` |
| a type | `jl.LinearAlgebra.Diagonal` |

Two packages that both extend `Base.show` share the entity `jl.Base.show`: that is Julia's meaning, one method table.
