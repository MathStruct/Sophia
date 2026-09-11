# Sea of Nodes

A graph-based intermediate representation (Cliff Click, 1995) in which a program is a single graph of value-producing nodes connected by *data-dependence* edges, with control flow represented by additional control edges rather than by an ordered list of instructions. Nodes are not placed in basic blocks until scheduling; until then they "float" — hence the name.

The payoff is that many optimisations become local graph rewrites. Global value numbering is just node hashing and deduplication; dead code elimination is unreachability from the graph's roots; code motion is free, because nothing has a position until the scheduler assigns one. The cost is that control dependence becomes implicit and subtle, debugging is harder, and the eventual scheduling step is doing real work that a block-structured IR gets for free.

Used in HotSpot's C2 compiler, in Graal/Truffle, and in V8's TurboFan (which has since moved to Turboshaft, a block-structured design — a relevant data point about the trade-off).

Sea of Nodes is the closest existing IR design to what [[Start Here]] proposes, and it is worth knowing about for three reasons:

1. It establishes that **a compiler can work directly on a graph** rather than on a linearised instruction sequence — this project's premise, validated in production compilers.
2. Its node-hashing-for-GVN is [[Hash Consing]] applied to an IR, which is [[Hashing and Identity]] in miniature.
3. The retreat from it in TurboFan is the main empirical argument *against* the premise: graph IRs are harder to reason about, harder to debug, and harder to keep fast than their advocates expect. Worth taking seriously in [[Open Problems and Risks]].

The main difference is scope: Sea of Nodes is an in-memory IR for one function during one compilation. Sophia proposes a *persistent, cross-function, cross-language, cross-session* graph — which is a much stronger claim and inherits all the same difficulties.

## Related

- [[SSA Form]]
- [[LLVM IR]]
- [[MLIR Operation]]
- [[Hash Consing]]
- [[State of the Art - Sea of Nodes and Graph IRs]]
