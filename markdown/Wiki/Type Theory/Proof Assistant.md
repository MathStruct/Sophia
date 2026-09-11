# Proof Assistant

A proof assistant (or interactive theorem prover) is a system in which a user constructs a formal, machine-checked proof, typically by building up a term whose type is the proposition to be proved (see [[Curry-Howard Correspondence]]) with the assistant's kernel checking every step. Well-known examples include Lean 4, Coq/Rocq, Agda, and Idris 2 — most of which are also usable as general-purpose dependently-typed programming languages (see [[Dependent Types]]).

Metamath, listed as a candidate "simple language" to prototype against in this project's README, is a related but more minimal system: rather than a rich dependently-typed kernel, it checks proofs as pure string-substitution derivations from axioms, which makes it an easy first target for parsing statements into a graph database before tackling something as large as Lean or Julia.

The relevance here is direct: "equivalency proofs" inserted into the graph database, as described in [[Start Here]], are exactly the kind of object a proof assistant's kernel knows how to check — the project effectively needs a proof-checking layer alongside its code-representation layer.

## Related

- [[Dependent Types]]
- [[Curry-Howard Correspondence]]
- [[Type Theory]]
- [[State of the Art - Dependent Types in Practice]]
