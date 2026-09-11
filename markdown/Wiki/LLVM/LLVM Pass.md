# LLVM Pass

A pass is a unit of transformation or analysis that runs over [[LLVM IR]]. Analysis passes compute information (e.g. which values alias, which loops exist) without changing the IR; transformation passes rewrite the IR (e.g. inlining, dead code elimination, loop unrolling, vectorization). A **pass manager** schedules passes into a pipeline, respecting dependencies between analyses and invalidating cached results when a transformation pass runs.

Optimization "levels" (`-O0` through `-O3`) are really just pre-built pipelines of many small, individually simple passes composed together.

For a graph-database-backed compiler, the pass concept generalizes naturally: a pass becomes a query (or a sequence of queries) over the graph that reads some subgraph and either annotates it or rewrites it — including inserting the kind of equivalency annotations described in [[Start Here]].

## Related

- [[LLVM IR]]
- [[LLVM]]
- [[MLIR Lowering]]
