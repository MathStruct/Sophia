# Design Overview

This folder is the *design* half of the vault. [[Wiki Home]] explains background concepts, [[State of the Art]] surveys what other people have built, and these notes state what **Sophia** itself is supposed to be: a content-addressed graph database of code, its types, its proofs and all of its intermediate representations, with a compiler that is expressed as queries over that database.

## The one-paragraph version

Every syntactic object — a declaration, a type, an expression, an MLIR operation, an LLVM instruction, a test, a doc comment, a proof — becomes a **node** keyed by a hash of its own normalised structure (see [[Hashing and Identity]]). Relationships between them — "has type", "lowers to", "depends on", "is proved equivalent to" — become **edges** (see [[Graph Schema]]). Frontends *elaborate* source languages into one common [[Core Calculus]]; backends *lower* core terms through [[MLIR]] into [[LLVM IR]]. Cross-language unification is not achieved by a calling convention but by storing [[Equivalence and Witnesses|equivalence edges]] carrying machine-checkable evidence. Compilation is then a sequence of queries and rewrites over the graph rather than a pass pipeline over a file (see [[Compilation as Query]]).

## The layer cake

```tikz
\usepackage{tikz-cd}
\begin{document}
\begin{tikzcd}[column sep=large, row sep=large]
\text{Julia} \arrow[dr, "\mathrm{elab}_J"'] & \text{C++} \arrow[d, "\mathrm{elab}_C"] & \text{Lean} \arrow[dl, "\mathrm{elab}_L"] \\
 & \text{Sophia Core} \arrow[d, "\mathrm{lower}"] & \\
 & \text{MLIR dialects} \arrow[d, "\mathrm{lower}"] & \\
 & \text{LLVM IR} \arrow[d, "\mathrm{llc}"] & \\
 & \text{machine code} &
\end{tikzcd}
\end{document}
```

Every arrow in that diagram is a *stored relation*, not a transient step inside a compiler process. If `elab_J` runs twice on the same input it produces the same hashes and writes nothing new; that is the whole point.

## Notes in this folder

| Note | What it settles |
| --- | --- |
| [[Repository Layout]] | Where code and docs live, and why |
| [[Core Calculus]] | The common language every frontend targets |
| [[Hashing and Identity]] | What a node's identity *is*, formally |
| [[Graph Schema]] | Node kinds, edge kinds, storage mapping |
| [[Equivalence and Witnesses]] | What "these two programs are the same" means and how strong the claim is |
| [[Compilation as Query]] | How a compiler falls out of a database |
| [[Multi-AST Layering]] | Terms, types and proofs as three views of one graph |
| [[Effects Memory and Resources]] | Effect rows, ownership, GC vs. manual memory |
| [[Cross-Language Semantic Hazards]] | The concrete ways "same thing" turns out to be false |
| [[Trusted Computing Base]] | What has to be correct for any of this to mean anything |
| [[Naming and Change Propagation]] | Names as mutable labels over immutable hashes |
| [[Tests and Documentation as Nodes]] | Non-compiled statements in the graph |
| [[Content-Addressed Precompilation]] | The original Julia motivation, made precise |
| [[Query Cookbook]] | Concrete queries in Cypher, Datalog and SQL |
| [[Roadmap]] | Milestones M0–M7 |
| [[Open Problems and Risks]] | Honest list of what may sink this |
| [[Glossary]] | Project vocabulary |

## Three claims this project is making

1. **Identity should be structural, not nominal.** Borrowed wholesale from [[Unison]] / [[Content-Addressed Code]]. This part is known to work.
2. **A compiler's intermediate state is worth persisting.** Nobody does this — compilers throw away every IR they build. Whether the query cost of reconstructing IR from a database beats rebuilding it from source is an open empirical question ([[Open Problems and Risks]]).
3. **Interop can be an assertion plus evidence, instead of an ABI.** This is the genuinely novel and genuinely risky claim; see [[Equivalence and Witnesses]] and [[State of the Art - Cross-Language Interoperability]].

## See also

- [[Start Here]] — the original statement of the idea
- [[Wiki Home]] — background concept cards
- [[State of the Art]] — prior art
