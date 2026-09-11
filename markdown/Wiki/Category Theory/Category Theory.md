# Category Theory

Category theory is the study of structures made of objects and directed arrows (morphisms) between them, plus a way of composing arrows, without needing to look inside the objects at all — everything is defined purely by how arrows compose. Because it operates at this high level of abstraction, the same categorical concepts recur across mathematics and, relevantly, across programming language theory.

The connection to this project runs through [[Curry-Howard-Lambeck Correspondence]]: cartesian closed categories model typed lambda calculi, functors model structure-preserving translations between type systems or between ASTs, and initial algebras of a functor model recursive data types and, by extension, syntax trees themselves. See [[Category Theory and Programming Languages]] for that connection in more detail.

For a project aiming to represent *multiple* ASTs (for types, for functions, for variables — see [[Start Here]]) and to relate them to each other formally, category theory supplies the standard vocabulary for saying precisely what "translating between two representations while preserving structure" means — that's what a [[Functor]] is.

## Related

- [[Category]]
- [[Functor]]
- [[Cartesian Closed Category]]
- [[Category Theory and Programming Languages]]
- [[Curry-Howard-Lambeck Correspondence]]
