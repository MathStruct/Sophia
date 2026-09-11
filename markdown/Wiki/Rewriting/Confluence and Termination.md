# Confluence and Termination

The two properties that decide whether a [[Term Rewriting System]] can be used as a decision procedure.

**Termination** (strong normalisation): there is no infinite chain $t_1 → t_2 → …$. Proved by exhibiting a well-founded measure that strictly decreases at every step (term size, a lexicographic path order, a polynomial interpretation). Undecidable in general.

**Confluence**: whenever $t →^* u$ and $t →^* v$, there is a `w` with $u →^* w$ and $v →^* w$. Pictured as the diamond:

```tikz
\usepackage{tikz-cd}
\begin{document}
\begin{tikzcd}[column sep=large, row sep=large]
 & t \arrow[dl, twoheadrightarrow] \arrow[dr, twoheadrightarrow] & \\
u \arrow[dr, twoheadrightarrow, dashed] & & v \arrow[dl, twoheadrightarrow, dashed] \\
 & w &
\end{tikzcd}
\end{document}
```

**Together they give unique normal forms**, and hence a decision procedure for the equational theory the rules generate: normalise both sides, compare syntactically. This is precisely the mechanism behind definitional equality in a proof assistant ([[Definitional vs Propositional Equality]]) and behind canonical-form hashing in [[Hashing and Identity]].

**Newman's lemma**: for a terminating system, *local* confluence (the diamond for single steps) implies confluence. This makes confluence checkable, because local confluence reduces to checking finitely many **critical pairs** — the overlaps between left-hand sides. **Knuth–Bendix completion** is the procedure that takes a set of equations and tries to orient them into a confluent terminating system by adding rules for critical pairs that do not join; it may loop forever, which is a fair summary of the general difficulty.

Why this matters for Sophia: the canonicaliser *is* a rewriting system, and it must be terminating and confluent, or hashing is nondeterministic and the entire store is unsound. That is the property the [[Roadmap|M0]] determinism test is checking for.

## Related

- [[Term Rewriting System]]
- [[E-Graph]]
- [[Normalization by Evaluation]]
- [[Hashing and Identity]]
- [[Trusted Computing Base]]
