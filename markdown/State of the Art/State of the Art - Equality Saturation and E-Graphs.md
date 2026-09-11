# State of the Art - Equality Saturation and E-Graphs

An **e-graph** (equivalence graph) is a data structure that stores many equivalent expressions in shared, compact form: nodes are grouped into equivalence classes, and an equivalence class can be referenced as an operand in place of any of its members. **Equality saturation** is the technique of repeatedly applying rewrite rules to an e-graph — never destructively replacing an expression, only *adding* the fact that two expressions are equivalent (merging their equivalence classes) — until no new equivalences are found, then extracting the best (e.g. cheapest, fastest) representative from each class.

This is, essentially, an existing and actively-researched implementation of the "insert equivalency proofs for two distinct UUIDs" idea in [[Start Here]], at the level of individual expressions rather than whole cross-language programs:

- **egg** (Rust, "e-graphs good") — a widely used, general-purpose e-graph library that made equality saturation practical for real compiler and synthesis workloads; used in projects ranging from numerical-precision tuning to CAD kernels to tensor-compiler optimization.
- **egglog** — merges e-graphs with Datalog, so rewrite rules are expressed as logical inference rules over a relational/graph database of terms — architecturally very close to "store program facts in a graph/relational database and derive equivalences by querying it," which is close to what [[Start Here]] describes wanting to build, but scoped to term rewriting rather than whole-program cross-language equivalence.
- Applications in optimizing compilers (e.g. tensor compilers using equality saturation instead of a fixed pass ordering, since equality saturation avoids the "phase ordering problem" of traditional pass pipelines like [[LLVM Pass|LLVM passes]]) — relevant as an alternative to a linear pass pipeline for the "compiler around the graph database" described in the README.

## Relevance

This body of work is the closest existing technical match to the equivalency-proof half of the project's goals, and is worth studying directly before designing that part of the schema — in particular, how e-graphs represent an equivalence class as a first-class graph node, and how egglog represents rewrite rules as queries.

## Related

- [[State of the Art - Content-Addressable Code Systems]]
- [[State of the Art - Proof-Carrying Code and Verified Compilation]]
- [[LLVM Pass]]
