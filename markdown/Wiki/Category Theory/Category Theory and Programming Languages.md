# Category Theory and Programming Languages

Beyond the [[Curry-Howard-Lambeck Correspondence]], category theory gives programming-language theory several tools that are directly applicable to a graph-database-of-code project:

- **Types-as-objects, programs-as-morphisms**: a language's type system forms a [[Category]] (or, for a well-behaved functional language, a [[Cartesian Closed Category]]), with function composition as categorical composition. This gives a formal notion of "does this translation preserve typing" for free.
- **Initial algebras and ASTs**: a recursive data type like an AST can be described as the *initial algebra* of a [[Functor]] representing its constructors (e.g. "a Node is either a Leaf, or an Op applied to two sub-Nodes"). This is the standard categorical account of "what is an AST," and it's exactly the multi-AST structure [[Start Here]] wants to store (one AST for expressions, one for types, one for variables) — each is the initial algebra of a different functor, and a *natural transformation* between functors is the categorical account of translating consistently between two such ASTs.
- **Monads and effects**: side effects (I/O, state, exceptions — compare [[Unison Abilities]]) are commonly modeled as monads, a specific kind of functor with extra structure, giving a language-agnostic vocabulary for the "or solve memory management" part of [[Start Here]]'s equivalency-annotation goal.

None of this is required to build the system, but it supplies precise, pre-existing vocabulary for claims the project wants to make informally, like "this Julia code and this C++ code do the same thing."

## Related

- [[Category Theory]]
- [[Functor]]
- [[Cartesian Closed Category]]
- [[Curry-Howard-Lambeck Correspondence]]
