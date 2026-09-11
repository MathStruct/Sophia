# Normalization by Evaluation

NbE computes the normal form of a term without repeatedly rewriting syntax. Instead it goes through a semantic domain and comes back:

$$ "norm" = "quote" ∘ "eval" quad quad "Syntax" ⇄ "Value" $$

`eval` interprets a term into host-language values, using host closures for lambdas, so β-reduction is just function application in the implementing language. `quote` (or *reify*) reads a semantic value back into syntax, generating fresh variables when it descends under a binder. Getting stuck on a free variable is handled by *neutral* terms, which is the mechanism that makes the round trip total rather than diverging on open terms.

NbE is the standard implementation of the conversion check in modern dependently-typed systems (Lean 4, Agda, `coqchk`, Idris 2, smalltt). It is dramatically faster than naive substitution-based reduction because substitution is never performed explicitly, and because the host's own environments and closures do the bookkeeping.

Two reasons it sits at the centre of Sophia's design:

1. **It decides definitional equality**, which is what the conversion rule of [[Core Calculus|SC]] needs. See [[Definitional vs Propositional Equality]].
2. **It produces the canonical form that gets hashed.** [[Hashing and Identity]] requires that definitionally equal terms have the same identity, so the canonicaliser's normalisation step *is* NbE. That places NbE squarely inside the [[Trusted Computing Base]]: a bug in `quote` does not merely produce a wrong answer, it silently merges two different definitions.

Note the interaction with [[De Bruijn Index|de Bruijn]] representations: quoting typically produces levels (stable as you descend) and converts to indices at the end.

## Related

- [[Definitional vs Propositional Equality]]
- [[De Bruijn Index]]
- [[Term Rewriting System]]
- [[Confluence and Termination]]
- [[Dependent Types]]
- [[Hashing and Identity]]
