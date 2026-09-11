# SSA Form

Static Single Assignment (SSA) form is a property of an intermediate representation where every variable is assigned exactly once, and every use of a variable is dominated by its single definition. When control flow merges (e.g. after an if/else), a special **phi node** selects which incoming definition to use based on which predecessor block control came from.

SSA makes data-flow analysis and many optimizations (constant propagation, dead code elimination, common subexpression elimination) simpler, because "the definition of this value" is unambiguous — there's exactly one place to look. Both [[LLVM IR]] and [[MLIR Operation|MLIR]] use SSA as their core representation.

SSA is notable here because it already pushes an IR toward being a graph rather than a sequence: each value is a node, and each use is an edge to the node that produced it. Storing such an IR in a graph database is a fairly direct translation.

## Related

- [[LLVM IR]]
- [[LLVM]]
- [[MLIR Operation]]
