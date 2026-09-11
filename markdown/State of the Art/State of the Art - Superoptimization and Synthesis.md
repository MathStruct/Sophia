# State of the Art - Superoptimization and Synthesis

If the database stores *many* equivalent implementations and picks the best one ([[Compilation as Query]]), then it is doing something close to superoptimisation. This note covers what that field has learned, since its results bound what Sophia's extraction step can realistically achieve.

- **Massalin's superoptimizer** (1987) — exhaustive search over short instruction sequences, testing equivalence against a specification. Established that machine-generated sequences routinely beat hand-written ones for small kernels, and that exhaustive search runs out of steam at around 4–6 instructions.
- **STOKE** (Stanford) — stochastic search (MCMC) over x86-64 programs, with correctness checked by a combination of testing and SMT verification. Found substantially faster implementations of real kernels, including some in production libraries. Demonstrates that the search space is navigable *without* exhaustiveness if the cost function is good.
- **Souper** — a superoptimizer for [[LLVM IR]] that extracts small expression slices, asks an SMT solver for a cheaper equivalent, and feeds results back as LLVM peephole rules. Directly relevant: it is a *rule harvester*, and its output is exactly the kind of witnessed rewrite rule [[Equivalence and Witnesses]] wants to store.
- **Denali** — superoptimisation using an E-graph and axioms, in 2002. The direct ancestor of [[Equality Saturation]] applied to code generation.
- **Rosette** — a solver-aided programming language (symbolic evaluation plus SMT) used to build synthesisers and verifiers for DSLs with modest effort. The practical toolkit if Sophia ever needs a bespoke equivalence checker for a sub-language.
- **Sketch** and **syntax-guided synthesis (SyGuS)** — program synthesis from a partial program plus a specification; SyGuS standardised the problem format and runs an annual competition.
- **Component-based synthesis / Hoogle-style search** — finding a program of a given type from a component library. Relevant because a store of hash-identified, typed definitions is *already* the index such a search needs.
- **Alive** — generates verified peephole optimisations from a DSL, closing the loop between synthesis and verification ([[State of the Art - Program Equivalence Checking]]).

## What this means for Sophia

1. **Automatically discovered equivalences are realistic at small scale.** Souper-style harvesting over stored core terms would populate `EQUIV` edges at `rewrite` level with machine-checkable witnesses and no human effort. This is the most credible answer to "nobody will write the proofs" in [[Open Problems and Risks]].
2. **Scale limits are real and well documented.** Superoptimisation works on expressions of a handful of operations, not on functions. Do not expect the e-graph to discover that a Julia sort and a C++ sort are equivalent; expect it to discover peepholes.
3. **The cost function matters more than the search.** STOKE's results came from a good objective. Sophia's extraction ([[Compilation as Query]]) depends on a cost model attached to the `Target` node, and getting that right is where the value is.
4. **A store of equivalent implementations is a synthesis index.** Once many implementations of the same core term are recorded, "find me the fastest implementation satisfying this specification on this target" is a database query rather than a search — which is arguably a *better* version of superoptimisation, because the search results are accumulated across everyone's builds instead of being recomputed.

Point 4 is the most attractive downstream consequence of the whole architecture, and it depends only on the parts that are already low-risk: hashing, storage, and benchmark nodes.

## Related

- [[Equality Saturation]]
- [[E-Graph]]
- [[Compilation as Query]]
- [[Equivalence and Witnesses]]
- [[State of the Art - Program Equivalence Checking]]
- [[State of the Art - Equality Saturation and E-Graphs]]
