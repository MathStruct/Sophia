# Term Rewriting System

A term rewriting system is a set of oriented rules $l → r$, applied by matching `l` against a subterm (finding a substitution σ with `σl` equal to that subterm) and replacing it with `σr`. It is the computational engine underneath most of what a compiler optimiser does, and underneath the conversion checker of a proof assistant.

Two properties are decisive for whether a rule set is usable:

- **Termination** — no infinite rewrite sequence. Commutativity (`a + b → b + a`) is the canonical non-terminating rule, which is why it cannot appear in an oriented system and must instead be handled by [[E-Graph|e-graphs]] or by matching modulo AC.
- **Confluence** — if a term can rewrite two different ways, the results can be rejoined. Confluence plus termination gives unique normal forms, which makes equality decidable by "normalise both sides and compare".

See [[Confluence and Termination]].

The relevance to [[Start Here]] is direct. A rewriting system with unique normal forms would let equality of programs be *decided* by normalisation, and hashing the normal form would make equivalent programs share an identity automatically. That is exactly what [[Hashing and Identity]] does for the fragment where it works — and the reason the fragment is small is that no terminating, confluent rule set captures program equivalence in general (it is undecidable). Everything outside that fragment needs explicit [[Equivalence and Witnesses|equivalence edges]].

## Related

- [[Confluence and Termination]]
- [[E-Graph]]
- [[Equality Saturation]]
- [[Normalization by Evaluation]]
- [[Hashing and Identity]]
