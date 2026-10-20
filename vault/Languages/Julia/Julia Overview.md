#overview #julia

> Julia in Sophia: the language that matters most, because Moonkale was built as Lenticulum.jl's editor (SR-114). Status by level ([[Languages]]), with the tests behind each claim. 2026-10-20.
>
> Sources: [[SPEC-008 Module-Level Julia Resolution]]; [[Real Code Corpora]] §5; Moonkale's `moonkale-content-id` (rev `dacef2e6`, normaliser v7); `crates/sophia-ingest`.

## Pages

| Page | Topic |
|---|---|
| [[Julia Parsing and Normalization]] | tree-sitter, WP-66 v7, what is refused |
| [[Julia Name Resolution]] | module scope, includes, imports, generic functions as (module, name) |
| [[Julia Multiple Dispatch and World Age]] | generic identity vs method table vs dispatch result |
| [[Julia Macros and Generated Functions]] | what Sophia sees of code that writes code |
| [[Julia Types]] | the type system as far as identity and dispatch need it |
| [[Julia AD and Compiler Backends]] | Zygote, Enzyme, Mooncake, Reactant: derived programs and artifacts |
| [[Julia Conformance]] | the conformance suite: Sophia against Julia itself |
| [[Julia SophiaLang Interoperability]] | how Julia definitions relate to sophialang |

## Status by level

| Level | Feature | Status | Evidence |
|---|---|---|---|
| L1 | top-level definitions: functions (long, short, `where`), structs, abstract and primitive types, constants, macros, modules | **yes** | tree-sitter-julia via arborium; LinearAlgebra: 6 of 3 552 definitions are malformed syntax |
| L2 | canonical form and content id | **yes**, identical to Moonkale | `tests/module.rs` reproduces every row of Moonkale's `module.tsv`; `tests/code.rs` the per-file fixtures |
| L2 | refusals | 136 of 3 552 in LinearAlgebra (27 unsupported syntax, 6 malformed, 103 depending on a refused one) | `examples/refusals.rs` |
| L3 | module-level references, across `include`d files | **yes**, as Moonkale | 2 469 references in LinearAlgebra, 948 across files |
| L3 | **generic-function ownership**: which function a method belongs to | **95.1 %** agree with Julia on LinearAlgebra (2 777 of 2 920); the rest are re-exported bindings | [[Julia Conformance]] |
| L3 | uses of generic functions (calls) | scope-aware since v6; 2 237 in LinearAlgebra | `examples/generic_calls.rs` |
| L3 | `using` bindings, re-exports (`Base.Array` is `Core.Array`) | **approximated** (not followed) | all 86 remaining ownership disagreements |
| L3 | macros, `@eval`, generated functions | **not resolved**: hashed as written; since v7 nothing inside a call is a unit | [[Julia Macros and Generated Functions]] |
| L3 | dispatch: which method a call reaches | **not attempted** | [[Julia Multiple Dispatch and World Age]] |
| L4 | lowering to sophialang or to Julia's lowered IR | no | — |
| L5 | execution | no; Julia runs Julia | — |
| L6 | verification | no | — |

## What Sophia stores for a package

`sophia ingest-code julia PKG DIR` in module mode ([[sophia_ingest]]):
- **records:** one per identified definition, with Moonkale's id;
- **generic functions:** nominal entities `jl.<module path>.<name>` with methods as members; methods of other modules' functions go to those (`jl.Base.show`);
- **types and constants:** named `jl.<module path>.<name>`;
- **references:** content references (in the id) and generic uses (`nominal_refs`, not in the id).

LinearAlgebra (Julia 1.13 stdlib, v7): 3 416 definitions with ids (identical to Moonkale), 3 284 distinct records, 798 generic functions (123 of them other modules' functions that LinearAlgebra extends) with 2 907 methods.

## The honest summary

Sophia can **identify** Julia definitions exactly as Moonkale does, and **mostly** knows which function a method belongs to. It does not know which method a call reaches, what a macro expands to, or what code runs. Those are the next semantic boundaries ([[Semantic Boundary Experiments]]), not more packages.
