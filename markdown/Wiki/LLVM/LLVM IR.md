# LLVM IR

LLVM IR is [[LLVM]]'s intermediate representation: a typed, low-level, RISC-like instruction set in [[SSA Form]]. It exists in three interchangeable forms — a human-readable textual format (`.ll`), an in-memory C++ data structure used by passes, and a compact binary serialization, [[LLVM Bitcode]].

Programs in LLVM IR are organized as modules containing functions, functions contain basic blocks, and basic blocks contain instructions that operate on typed values (integers, floats, pointers, vectors, aggregates). Control flow between blocks is explicit, and data dependencies between instructions are explicit — which makes the IR close to a directed acyclic graph of value-producing nodes, rather than a flat instruction stream.

That graph-like structure is one reason IRs like this are a natural fit for representation in a graph database: instructions already reference their operands by identity rather than by name.

## Related

- [[LLVM]]
- [[SSA Form]]
- [[LLVM Bitcode]]
- [[LLVM Pass]]
- [[MLIR Operation]]
