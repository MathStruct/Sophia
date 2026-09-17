# Operational Semantics

A definition of what a program *does*, given as inference rules describing execution steps on syntax. Two standard styles:

**Small-step** (structural, Plotkin): a relation $t → t'$ on configurations, one reduction at a time.

$$ ((t_1 → t_1')) / (t_1 space t_2 → t_1' space t_2) quad quad (lambda x. t) space v → t[v slash x] $$

**Big-step** (natural, Kahn): a relation $t ⇓ v$ directly relating a term to its final value.

Small-step is preferred when non-termination and interleaving matter — you can talk about a program that runs forever, and about concurrency, both of which big-step semantics handle awkwardly. Big-step is shorter for deterministic terminating languages.

The relevance here is that **you cannot state an equivalence claim without a semantics**. [[Contextual Equivalence]] is defined in terms of $⇓$; [[Bisimulation]] is defined in terms of $→$. So "this Julia code does the same as this C++ code" presupposes a $→$ or $⇓$ for both languages — and for Julia no such relation has ever been written down. See [[State of the Art - Formal Semantics of Real Languages]] and the corresponding entry in [[Open Problems and Risks]].

Sophia's practical response is to give the operational semantics for [[Core Calculus|SC]] only, and require frontends to elaborate into it — so there is one semantics rather than `n`, and the per-language difficulty is pushed into the elaboration, where it is at least visible.

## Related

- [[Contextual Equivalence]]
- [[Bisimulation]]
- [[Core Calculus]]
- [[State of the Art - Formal Semantics of Real Languages]]
