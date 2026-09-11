# MLIR Operation

`Operation` is the single, generic unit that everything in [[MLIR]] is built from — there is no separate "instruction" or "function" class baked into the core IR. Every operation has: a name (namespaced by its [[MLIR Dialect|dialect]], e.g. `arith.addi`), a list of typed operand values it consumes, a list of typed result values it produces, a set of compile-time attributes, and optionally one or more nested **regions**, each containing further blocks of operations.

Because operands and results are SSA values (see [[SSA Form]]) referenced by identity, and regions let operations nest arbitrarily (a loop is an operation containing a region containing more operations), an MLIR module is already, structurally, a directed graph with hierarchical nesting — not a flat list of instructions. This maps unusually directly onto a graph-database schema: operations as nodes, operand/result edges as data-flow edges, and region containment as a parent/child edge.

## Related

- [[MLIR]]
- [[MLIR Dialect]]
- [[SSA Form]]
- [[LLVM IR]]
