# State of the Art - Proof-Carrying Code and Verified Compilation

Approaches to attaching machine-checkable correctness evidence to code, rather than trusting the compiler or runtime:

- **Proof-Carrying Code (PCC)**, George Necula's 1990s work — a producer ships compiled code *together with* a formal proof that it satisfies a given safety policy; the consumer's proof checker verifies the proof (cheap) instead of re-deriving trust some other way (e.g. sandboxing or a trusted compiler). Conceptually the closest historical ancestor of "inserting equivalency proofs" as first-class citizens alongside code, from [[Start Here]].
- **CompCert** — a C compiler formally verified (in Coq/Rocq) to preserve program semantics from source to assembly; the proof shows the generated code is a correct translation of the source, for every input program, rather than checking each compilation individually. Demonstrates translation-equivalence proofs at compiler scale, but for one fixed source and target language pair, verified once for the whole compiler rather than inserted per-statement into a database.
- **Translation validation** — a lighter-weight alternative to a fully verified compiler: instead of proving the compiler correct once and for all, check *after each individual compilation* that the output is equivalent to the input (often using techniques overlapping with [[State of the Art - Equality Saturation and E-Graphs]]). This is architecturally closer to what [[Start Here]] describes: proofs generated and checked per translation instance, stored alongside the specific statements they relate, rather than one global compiler-correctness theorem.
- **Metamath**, listed in the README as a candidate first target language — a minimal formal system where a proof is literally a sequence of substitutions from axioms; useful precisely because it's small enough to prototype "parse a proof language into a graph database" against before attempting a full proof assistant like Lean (see [[Proof Assistant]]).

## Relevance

Translation validation is probably the most directly reusable idea: rather than proving a whole "Julia-to-C++ equivalence compiler" correct in general, the project could check and store an equivalence proof for each specific pair of definitions as they're added to the graph, which fits its incremental, per-statement, hash-addressed model much better than a monolithic verified-compiler approach.

## Related

- [[State of the Art - Equality Saturation and E-Graphs]]
- [[Proof Assistant]]
- [[Curry-Howard Correspondence]]
