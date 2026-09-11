# Curry-Howard-Lambeck Correspondence

The Curry-Howard-Lambeck correspondence extends [[Curry-Howard Correspondence|Curry-Howard]] with a third leg: category theory. Joachim Lambek showed that the simply typed lambda calculus corresponds not only to intuitionistic propositional logic (proofs) but also to [[Cartesian Closed Category|cartesian closed categories]] (a class of categories with enough structure — products and exponential objects — to model function types). The three corners of the correspondence are:

- **Logic**: propositions and proofs
- **Type theory**: types and terms/programs
- **Category theory**: objects and morphisms in a cartesian closed category

Each side gives a different, equally valid way to talk about the same underlying structure. This three-way view is what [[Start Here]] references directly: "there is the Curry-Howard-Lambeck correspondence; we would like to build multiple ASTs, for types, functions, variables." The implication is that a program's *types*, its *functions/terms*, and the *proofs about it* aren't three unrelated things to store separately — they're three views of one structure, which argues for representing them as compatible, cross-referencing layers within the same graph rather than as separate, disconnected artifacts.

## Related

- [[Curry-Howard Correspondence]]
- [[Cartesian Closed Category]]
- [[Category Theory]]
- [[Type Theory]]
