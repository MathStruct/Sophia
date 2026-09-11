# State of the Art - Dependent Types in Practice

[[Start Here]] traces back to wanting "Julia + Lean combined, i.e. a Julia with dependent types." Current dependently-typed systems, for context:

- **Lean 4** — currently the most actively growing proof assistant/dependently-typed language, notable for being self-hosted (its compiler and much of its tooling are written in Lean itself), having a large and rapidly growing formalized-mathematics library (Mathlib), and being designed from the start to also work as a general-purpose *programming* language with good performance, not only a proof language. Its combination of "usable as a real language" and "has dependent types and tactics" is the closest existing thing to the "Julia with dependent types" goal, and is a natural candidate to interoperate with or draw design ideas from, per [[Proof Assistant]].
- **Idris 2** — a general-purpose programming language built around full dependent types from the ground up, with a particular focus on making dependently-typed *programming* (not just proving) ergonomic, and on quantitative type theory (tracking how many times a value is used, related to linear/affine types).
- **Agda** — primarily a proof assistant / research vehicle for dependent type theory, widely used in programming-language-theory research; less focused on being a practical general-purpose language than Lean or Idris.
- **Coq / Rocq** — one of the most mature proof assistants, used for large verification efforts (including CompCert, see [[State of the Art - Proof-Carrying Code and Verified Compilation]]); recently renamed to Rocq.
- **F\*** — a dependently-typed language designed specifically for program *verification*, with an SMT-solver-backed tactic style (rather than only interactive proof terms) and a track record of being used to verify real cryptographic and systems code, extracted to OCaml or C.

## Relevance

None of these are dynamically-typed languages with dependent types retrofitted on — they're all dependently-typed from the ground up. Retrofitting dependent types (or a dependent-type-checked proof layer) onto Julia, as opposed to building a new dependently-typed language from scratch, appears to be genuinely underexplored territory, which lines up with the README's own uncertainty about whether this goal is "viable."

## Related

- [[Dependent Types]]
- [[Proof Assistant]]
- [[Curry-Howard Correspondence]]
- [[State of the Art - Julia Compilation and Precompilation]]
