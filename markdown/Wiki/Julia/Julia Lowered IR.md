# Julia Lowered IR

The representation Julia code takes after *lowering* — desugaring — and before type inference. Reachable from the REPL with `Meta.lower(mod, expr)` or `@code_lowered`, and represented as a `CodeInfo` object.

Lowering desugars everything syntactic into a small core: `for` becomes `iterate` calls, `do` blocks become anonymous functions, destructuring becomes `indexed_iterate`, comprehensions become generator closures, operators become function calls, and control flow becomes explicit `goto`/`gotoifnot` on a flat statement vector with `SSAValue` and `SlotNumber` operands. It is not quite [[SSA Form|SSA]] — slots are mutable — but the later typed IR (`@code_typed`, after inference) is.

For Sophia this is the natural ingestion point for the Julia frontend, and the reason is that **it is the last representation before type inference and dispatch resolution**. Ingesting surface syntax would mean reimplementing Julia's desugaring; ingesting typed IR would mean the inference results are baked in and the ingestion is specific to one argument-type tuple. Lowered IR is generic, canonical, and produced by Julia itself rather than by a reimplementation.

The pipeline positions, for reference:

| Stage | Access | What it has |
| --- | --- | --- |
| surface AST | `Meta.parse` | macros unexpanded, all sugar present |
| lowered | `@code_lowered` | desugared, untyped, slot-based, explicit control flow |
| typed / inferred | `@code_typed` | one specialisation, inferred types, inlining decisions |
| LLVM IR | `@code_llvm` | [[LLVM IR]] |
| native | `@code_native` | machine code |

A frontend should record `ELABORATES_TO` edges from lowered IR to [[Core Calculus|SC]], and — separately — the *typed* IR per specialisation together with its [[World Age|dispatch facts]], since that is what [[Content-Addressed Precompilation]] needs to key the code cache on.

## Related

- [[Multiple Dispatch]]
- [[World Age]]
- [[SSA Form]]
- [[Frontend]]
- [[State of the Art - Julia Compilation and Precompilation]]
