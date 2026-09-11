# Multiple Dispatch

Julia selects which method of a generic function to run based on the runtime types of **all** arguments, not just the first. `+(::Int, ::Float64)`, `+(::Matrix, ::Matrix)` and `+(::Missing, ::Any)` are separate methods of one function `+`, and applicability is decided by Julia's subtyping relation over a lattice of types including unions, tuples, parametric types and `Union{}`.

Dispatch is the organising principle of the ecosystem: packages extend each other's functions with methods for their own types, which is why so much Julia code composes without either package knowing about the other. It is also the reason the compiler specialises per concrete argument-type tuple, which is where both the performance and the latency come from ([[State of the Art - Julia Compilation and Precompilation]]).

Three properties make it hard for a system like Sophia:

- **Open world.** Any package loaded later may add a more specific method, changing which method a call site resolves to. A definition's meaning is therefore relative to a [[World Age|world age]].
- **Dispatch is semantic, not syntactic.** Which method runs is not determined by the call syntax; the graph must record *dispatch facts* ("at this call site, with these inferred argument types, the applicable method was `m`") as explicit dependency nodes. See [[Content-Addressed Precompilation]].
- **Ambiguity is a runtime error**, and whether two methods are ambiguous depends on the whole loaded method table.

Contrast C++ templates and Rust traits, which are resolved at compile time under a closed world. That mismatch is one of the concrete obstacles listed in [[Cross-Language Semantic Hazards]]: a claim about a Julia generic function is implicitly conditioned on a method table, while the corresponding C++ claim is not conditioned on anything.

## Related

- [[World Age]]
- [[Julia Lowered IR]]
- [[Content-Addressed Precompilation]]
- [[Cross-Language Semantic Hazards]]
- [[State of the Art - Julia Compilation and Precompilation]]
