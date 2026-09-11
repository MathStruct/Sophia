# Natural Transformation

A natural transformation is a morphism *between [[Functor|functors]]*. Given $F, G : cal(C) → cal(D)$, it is a family of morphisms $eta_X : F(X) → G(X)$, one for each object `X` of `C`, such that for every morphism $f : X → Y$ the square commutes:

```tikz
\usepackage{tikz-cd}
\begin{document}
\begin{tikzcd}[column sep=huge, row sep=large]
F(X) \arrow[r, "\eta_X"] \arrow[d, "F(f)"'] & G(X) \arrow[d, "G(f)"] \\
F(Y) \arrow[r, "\eta_Y"'] & G(Y)
\end{tikzcd}
\end{document}
```

The commuting condition is the entire content: the transformation must work *uniformly*, not case by case. Translate-then-map must equal map-then-translate.

In programming terms, a polymorphic function `∀a. F a -> G a` that does not inspect `a` is automatically natural — this is the content of the "theorems for free" parametricity result, and it is why `reverse :: [a] -> [a]` commutes with `map f`.

For [[Multi-AST Layering|Sophia]] this is not decoration. If frontends are functors $F_"Julia", F_"C++" : cal(P) → cal(C)$ from a shared interface category into [[Core Calculus|SC]], then a *coherent* set of cross-language equivalence claims is exactly a natural transformation $eta : F_"Julia" ⇒ F_"C++"$. Naturality says: having claimed `jl_sort ≈ cpp_sort` and `jl_map ≈ cpp_map`, the equivalence of their composites *follows* rather than needing its own assertion.

That is the formal version of the congruence requirement in [[Equivalence and Witnesses]], and the reason a finite set of claims can be useful for an unbounded set of programs. It also gives a concrete test: the naturality squares are enumerable, so a set of asserted equivalences can be *checked for coherence* rather than merely accumulated.

## Related

- [[Functor]]
- [[Category]]
- [[Initial Algebra]]
- [[Multi-AST Layering]]
- [[Equivalence and Witnesses]]
