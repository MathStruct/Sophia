# Curry-Howard Correspondence

The Curry-Howard correspondence ("propositions as types") is the observation that a logical proposition and a type are the same kind of object, and a proof of that proposition and a program (term) of that type are the same kind of object. Logical implication corresponds to a function type; conjunction corresponds to a product (tuple) type; disjunction corresponds to a sum (variant) type; and proving a proposition corresponds to *constructing* a term of the corresponding type — type-checking the term is checking the proof.

This is the theoretical justification for treating "equivalency proofs between code" ([[Start Here]]) as things that can live in the same graph, as the same kind of node, as the code itself: a proof that two functions are equivalent is a term whose type is the proposition "these two functions are equivalent," constructed the same way any other typed program is.

Extended with categorical semantics, this becomes the [[Curry-Howard-Lambeck Correspondence]].

## Related

- [[Type Theory]]
- [[Dependent Types]]
- [[Curry-Howard-Lambeck Correspondence]]
- [[Proof Assistant]]
