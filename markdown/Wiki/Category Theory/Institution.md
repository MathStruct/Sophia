# Institution

An institution (Goguen and Burstall, 1984) is a formalisation of "a logic", abstract enough that one can reason about translating *between* logics. It consists of:

- a category **Sign** of signatures and signature morphisms,
- a functor **Sen** giving, for each signature, its set of sentences,
- a functor **Mod** giving, for each signature, its category of models,
- a satisfaction relation $⊨_Sigma$ between models and sentences,

subject to the **satisfaction condition**: for any signature morphism $sigma : Sigma → Sigma'$, any $Sigma'$-model `M'` and any $Sigma$-sentence `φ`,

$ M' ⊨_(Sigma') "Sen"(sigma)(phi) quad ⟺ quad "Mod"(sigma)(M') ⊨_Sigma phi $

In words: **truth is invariant under change of notation.**

Institutions were developed for algebraic specification (Clear, CASL, and the Hets toolset, which implements dozens of logics and the translations between them), precisely to allow a specification written in one logic to be reused in another with its meaning preserved.

The relevance to [[Start Here]] is direct, and it is the most useful "someone has already thought about this" pointer in the vault. A Sophia frontend is meant to be a meaning-preserving translation from a source language into [[Core Calculus|SC]] — which is exactly an **institution comorphism**. The satisfaction condition is the formal statement of the obligation a frontend must discharge: a property proved about the elaborated core term must be a property of the original program, and vice versa. [[Multi-AST Layering]] states the same requirement in functorial language; institutions state it for the *logical* content rather than just the syntactic structure, which is what is actually needed once proofs enter the picture.

Whether the machinery buys anything practical is genuinely unclear — it is a specification-level formalism, not an implementation technique. What it buys for certain is vocabulary, and the knowledge that "many languages, one store, meaning preserved" has a forty-year-old mathematical treatment.

## Related

- [[Functor]]
- [[Natural Transformation]]
- [[Multi-AST Layering]]
- [[Equivalence and Witnesses]]
- [[Category Theory and Programming Languages]]
