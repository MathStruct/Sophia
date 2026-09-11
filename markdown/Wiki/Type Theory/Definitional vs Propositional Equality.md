# Definitional vs Propositional Equality

Type theory has two distinct notions of "equal", and confusing them is one of the classic sources of difficulty in dependently-typed programming. The distinction is also the structural backbone of [[Hashing and Identity|Sophia's design]].

**Definitional (judgemental) equality**, $Gamma ⊢ t ≡ u$, is a *judgement*: the typechecker decides it silently, by normalising both sides ([[Normalization by Evaluation]]) and comparing. `2 + 2 ≡ 4` holds definitionally because both reduce to the same numeral. It is decidable (in a normalising theory), it requires no evidence, and it is used automatically by the conversion rule:

$ ((Gamma ⊢ t : A) quad (Gamma ⊢ A ≡ B)) / (Gamma ⊢ t : B) $

**Propositional equality**, $t =_A u$, is a *type*. Inhabiting it requires a proof term, obtained by `refl` when the two sides happen to be definitionally equal, and otherwise by actual reasoning (induction, rewriting, a tactic). `n + 0 = n` for a variable `n` is propositional, not definitional, if `+` recurses on its first argument — the canonical example of the two notions coming apart.

## Why Sophia is built on this split

| | Definitional | Propositional |
| --- | --- | --- |
| Decided by | kernel, silently | a stored proof term |
| Evidence | none needed | a [[Equivalence and Witnesses\|Witness]] node |
| In Sophia | **folded into the hash** — same identity | an `EQUIV` edge between two hashes |

That is the whole architecture in two rows. Everything the canonicaliser can normalise away becomes *identity* and costs nothing to exploit; everything it cannot becomes an *edge* carrying evidence, with a strength level and a `modulo` set. The design question "how aggressive should canonicalisation be" is exactly the question "how much equality should be definitional", and it has the same trade-off as in type theory: more definitional equality means more things work automatically, and a more complex, more fragile, more trusted kernel.

The further reaches of this question — extensionality, `Prop` vs `Type`, univalence, setoid hell, cubical computation of transport — are the subject of decades of work in type theory, and they apply here unchanged.

## Related

- [[Normalization by Evaluation]]
- [[Curry-Howard Correspondence]]
- [[Dependent Types]]
- [[Hashing and Identity]]
- [[Equivalence and Witnesses]]
