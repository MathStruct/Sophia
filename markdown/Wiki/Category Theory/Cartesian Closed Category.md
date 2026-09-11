# Cartesian Closed Category

A cartesian closed category (CCC) is a [[Category]] that has enough extra structure to model typed functional programming: a terminal object (a type with exactly one value, like `unit`), binary products (pair/tuple types), and exponential objects (function types `A → B`, with the universal property that makes currying well-defined).

CCCs matter here as the categorical corner of the [[Curry-Howard-Lambeck Correspondence]]: the simply typed lambda calculus, intuitionistic propositional logic, and cartesian closed categories are three equivalent presentations of the same structure. Concretely, a CCC's objects are types, its morphisms are (equivalence classes of) programs/proofs between those types, and composition is function composition — which is another way of saying "a typed programming language is a category," giving a formal grounding for organizing a graph database of typed code categorically.

## Related

- [[Category]]
- [[Curry-Howard-Lambeck Correspondence]]
- [[Category Theory]]
