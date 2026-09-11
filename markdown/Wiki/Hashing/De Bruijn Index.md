# De Bruijn Index

A de Bruijn index replaces a bound variable's name with a natural number counting how many binders lie between the variable's occurrence and the binder that introduced it. `λx. λy. x` becomes `λ. λ. 1`, and `λx. λy. y` becomes `λ. λ. 0`.

The representation makes [[Alpha Equivalence|α-equivalence]] into syntactic identity, which is exactly what a hashing scheme needs. It also makes substitution mechanical — at the cost of requiring index *shifting* whenever a term moves under a binder, which is the classic source of off-by-one bugs in implementations.

Two refinements are standard in practice:

- **De Bruijn levels** count from the *outside* in rather than the inside out. Levels are stable under moving a subterm deeper, indices are stable under moving a context around it; implementations often keep both and convert.
- **Locally nameless**: bound variables use indices, free variables keep names (or, in Sophia's case, the *hash* of the definition they refer to). This avoids shifting the free variables and is the representation [[Hashing and Identity]] assumes.

## Related

- [[Alpha Equivalence]]
- [[Core Calculus]]
- [[Normalization by Evaluation]]
- [[Hashing and Identity]]
