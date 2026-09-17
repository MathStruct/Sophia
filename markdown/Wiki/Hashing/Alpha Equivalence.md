# Alpha Equivalence

Two terms are α-equivalent, written $t ≡_alpha u$, if they differ only in the *names* of bound variables. `λx. x` and `λy. y` are the same function; the names are artefacts of how the term was written down.

For a system that identifies code by hashing its syntax, α-equivalence is the first normalisation that must happen. If it does not, renaming a loop counter changes a definition's identity and every dependent's identity with it — which defeats the purpose. The requirement is:

$$ t ≡_alpha u quad ⟹ quad h(t) = h(u) $$

The standard implementation is to eliminate names for bound variables entirely, using [[De Bruijn Index|de Bruijn indices]] (or a *locally nameless* representation: indices for bound variables, hashes for free ones). Then α-equivalent terms are *structurally identical*, and the implication above holds by construction rather than by a separate check.

Note that the converse is deliberately not claimed. Many pairs of terms are semantically equal without being α-equivalent; those need [[Definitional vs Propositional Equality|definitional equality]] or an [[E-Graph|equivalence edge]].

## Related

- [[De Bruijn Index]]
- [[Hashing and Identity]]
- [[Definitional vs Propositional Equality]]
- [[Content-Addressed Code]]
