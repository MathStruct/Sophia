#definition #overview

> Sources: background card
>
> Theory (CT-ML wiki): [Curry-Howard-Lambek Correspondence](https://mathstruct.org/CategoryTheory-ML-Wiki/Curry-Howard-Lambek-Correspondence) · [Category with Families](https://mathstruct.org/CategoryTheory-ML-Wiki/Category-with-Families)

Type theory studies systems that classify terms (expressions, values, proofs) by types, and the rules for what operations are valid on a term given its type. It began as a foundation for mathematics alternative to set theory (Russell, then Church's simply typed lambda calculus), and has since become the theoretical backbone of statically-typed programming languages and machine-checked proof systems alike.

The idea directly relevant to [[The Original Idea]] is [Curry-Howard](https://mathstruct.org/CategoryTheory-ML-Wiki/Curry-Howard-Lambek-Correspondence): a typed program and a mathematical proof are, formally, the same kind of object. That correspondence is what makes it sensible to talk about "inserting equivalency proofs" into a database of program statements — a proof that two pieces of code are equivalent is itself just another typed term, of the same general kind as the code.

[[Dependent Types]] extend this further by letting types themselves depend on runtime values, which is the mechanism behind proof assistants like Lean, Agda, and Idris — see [[Proof Assistant]].

## Related

- [[Dependent Types]]
- [Curry-Howard Correspondence](https://mathstruct.org/CategoryTheory-ML-Wiki/Curry-Howard-Lambek-Correspondence)
- [[Proof Assistant]]
- [the CT-ML wiki](https://mathstruct.org/CategoryTheory-ML-Wiki/)
