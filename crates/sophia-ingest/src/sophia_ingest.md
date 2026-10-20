#implementation

> Frontends that bring real code into Sophia (B-16, [[Real Code Corpora]]). Metamath first. Source: `sophia_ingest.rs`.
>
> Sources: [[Real Code Corpora]]; [[SPEC-006 Worlds, Artifacts and Evidence]] §1. code: `sophia_ingest.rs`, `metamath.rs`

`store_metamath` stores a parsed database:
- **statements:** one record each, named `mm.<db>.<label>`;
- **proofs:** one record each, named `mmproof.<db>.<label>`;
- **links:** a `proof` annotation on each statement and its proof;
- **the library:** three nominal collections, `mmlib.<db>.proofs`, `.axioms` and `.definitions`, filled with `Store::add_members`.

Being an axiom, or the proof chosen for a statement, is a **library's** choice, not a property of the statement. One statement can be an axiom in one library and proved in another, or even in another part of the same library: set.mm's personal sections prove `ax-1` from other axiom systems.

`examples/mm_overlap.rs` compares set.mm and iset.mm (statements and proofs shared).

`store_code` and `store_module` store code (Julia, Rust, Python) through `store_items`:
- **records** with Moonkale's ids;
- **generic functions** keyed by **(module path, name)**, never by name alone (`binding`, the one grouping rule; [[Julia Name Resolution]]):
  - a Julia method belongs to `extends` when it has one (`Base.show(…) = …`, `A.f(…) = …`, or `show(…) = …` after `import Base: show`), else to its own module's function;
  - the entity is `jl.<module path>.<name>`, so `jl.LinearAlgebra.dot` and `jl.LinearAlgebra.BLAS.dot` are two functions, and `jl.Base.show` holds the package's methods of Base's `show`;
  - a function named like a type in the same module is its constructor: `jl.<module>.<T>.new`;
  - a qualified method that cannot be placed (`Base.$f(…)` in an `@eval` loop) belongs to no generic function;
  - outside Julia, the module is the package name;
- **named definitions** (types, constants, macros) as `jl.<module path>.<name>`;
- **generic uses** as `nominal_refs`, keyed the same way (the crate resolves a use in its own module).

`CodeReport::extended` counts the generic functions of other top-level modules (Base's, mostly) that the package adds methods to. Tested by `tests/generics.rs` on `tests/julia-semantics/generics`; checked against Julia itself by `examples/julia_conformance.rs`.
