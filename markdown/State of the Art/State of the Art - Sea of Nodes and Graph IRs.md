# State of the Art - Sea of Nodes and Graph IRs

[[Start Here]] proposes storing code *as a graph* and compiling from it. Production compilers have done a version of this for thirty years, at a smaller scope, and their experience is the most directly applicable empirical evidence available.

- **[[Sea of Nodes]]** (Cliff Click, 1995) — a single graph of value nodes with data- and control-dependence edges and no fixed instruction order until scheduling. Shipped in HotSpot's C2 compiler. Global value numbering is node hashing; dead code elimination is unreachability; code motion is free. This is the strongest existing demonstration that a compiler can operate on a graph rather than an instruction list.
- **Graal / Truffle** — a modern sea-of-nodes JIT, with the added relevance that Truffle is also the polyglot runtime discussed in [[State of the Art - Cross-Language Interoperability]]. The same project therefore embodies both halves of this vault's interests, at runtime rather than in a database.
- **V8 TurboFan → Turboshaft** — TurboFan was sea-of-nodes; the V8 team has been migrating to Turboshaft, a **block-structured** IR, citing debuggability, compile-time predictability and the difficulty of reasoning about floating control. This is the most important negative data point in this note and should not be skipped over.
- **[[MLIR Operation|MLIR]]** — operations with typed operands/results and nested regions. Graph-structured but explicitly *block-and-region structured* rather than floating, i.e. deliberately positioned between the two extremes.
- **Cranelift CLIF** — block-structured [[SSA Form|SSA]] with e-graph-based mid-end optimisation ("ægraphs"), which is a notably close analogue of [[Compilation as Query|Sophia's plan]]: keep the block structure, but use an [[E-Graph|e-graph]] for the rewriting.
- **Program Dependence Graph** (Ferrante, Ottenstein, Warren) and the **Value Dependence Graph** (Weise et al.) — the academic ancestors, and still the standard citations for "represent a program as a dependence graph".
- **Firm / libFirm** — a research compiler built entirely on a graph-based SSA IR, with an unusually complete published account of the design.

## What this body of work establishes

1. Graph IRs work, in production, for real optimisation.
2. Node hashing for deduplication ([[Hash Consing]]) is standard practice, not an exotic idea — [[Hashing and Identity]] is the same move made persistent and cryptographic.
3. E-graphs in a real compiler mid-end are now shipping (Cranelift), which de-risks a substantial piece of [[Compilation as Query]].

## What it does not establish, and the caution

All of these are **in-memory, single-function, single-compilation** graphs. None persists the graph, none shares it across processes or machines, none stores multiple languages in one graph, and none attaches proofs to it. Every one of those extensions is where Sophia's actual risk lives.

And the V8 migration away from sea-of-nodes is a genuine warning: the team with the most production experience of a floating graph IR concluded that the block-structured alternative was easier to work with. The lesson is not "don't build a graph IR" — MLIR and Cranelift are thriving — but "keep explicit structure where you can", which argues for Sophia's layered [[Core Calculus|core]] → MLIR → LLVM design over a single undifferentiated sea of nodes.

## Related

- [[Sea of Nodes]]
- [[Compilation as Query]]
- [[MLIR Operation]]
- [[State of the Art - Equality Saturation and E-Graphs]]
- [[Open Problems and Risks]]
