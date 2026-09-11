# World Age

Julia's method tables are mutable at runtime — defining a method is an ordinary side effect. **World age** is the mechanism that keeps this from making compilation impossible: a global counter is incremented on every method-table change, every method records the world range in which it is valid, and every running task executes against a fixed world age.

The practical consequence is the familiar error: a function that defines a method with `eval` cannot call that method in the same invocation, because the caller is executing in an older world. `Base.invokelatest` is the escape hatch.

The reason it exists is compilation soundness. Julia devirtualises and inlines based on the method table it sees. If a more specific method appears later, previously compiled code that baked in the old resolution is *wrong* and must be invalidated. World age is what makes "when was this valid" a well-defined question, and **backedges** are the in-memory structure recording which compiled code depends on which dispatch decisions.

This is directly load-bearing for Sophia:

- Every equivalence or compilation claim about a Julia generic function is implicitly **relative to a world age**. [[Equivalence and Witnesses]] edges touching Julia code must record it, or the claim is unconditioned and false.
- [[Content-Addressed Precompilation]] proposes persisting dispatch facts as first-class `DependsOn` nodes — essentially, making Julia's ephemeral backedges into durable, queryable graph data. That is what would allow invalidation to be computed *exactly*, per-specialisation, instead of at package granularity, and it is the concrete claim that milestone is meant to test.

## Related

- [[Multiple Dispatch]]
- [[Content-Addressed Precompilation]]
- [[Pkgimage]]
- [[State of the Art - Julia Compilation and Precompilation]]
