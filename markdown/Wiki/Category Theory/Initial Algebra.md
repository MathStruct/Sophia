# Initial Algebra

Given an endofunctor $F : cal(C) → cal(C)$, an **F-algebra** is an object `A` together with a morphism $alpha : F(A) → A$. The **initial** F-algebra is the one with a unique morphism to every other F-algebra.

This is the categorical account of a recursive data type — and therefore of an abstract syntax tree. Take

$$ F(X) = "Lit" × bb(Z) + "Add" × X × X + "Mul" × X × X $$

Its initial algebra is exactly the type of arithmetic expression trees, and the unique morphism out of it is the **catamorphism** (`fold`): giving an interpretation of each constructor uniquely determines an interpretation of every tree. Lambek's lemma adds that the structure map of an initial algebra is an isomorphism, $F(mu F) ≅ mu F$ — the formal statement of "an AST is exactly one layer of constructors wrapped around more ASTs".

Why this matters for a graph database of code:

- **Each AST layer is the initial algebra of a different functor** — one for surface Julia, one for [[Core Calculus|SC]] terms, one for [[MLIR Operation|MLIR ops]]. [[Multi-AST Layering]] says exactly this.
- **Every fold over an AST is a catamorphism**, including the hash. $h : mu F → "Hash"$ is the catamorphism induced by the algebra "combine the tag with the children's hashes", which is precisely [[Merkle DAG|Merkle hashing]] — and its uniqueness is the formal reason the hash is well-defined and deterministic.
- **The dual, the final coalgebra**, models infinite/cyclic structures. That the hash's well-definedness relies on initiality is exactly why cycles break it and need [[Cycle Hashing|separate treatment]].

## Related

- [[Functor]]
- [[Natural Transformation]]
- [[Category Theory and Programming Languages]]
- [[Merkle DAG]]
- [[Multi-AST Layering]]
