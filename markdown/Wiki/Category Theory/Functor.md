# Functor

A functor is a structure-preserving map between two [[Category|categories]]: it sends every object of the source category to an object of the target category, and every morphism to a morphism, in a way that preserves composition and identities. Informally: it's a translation that doesn't break the shape of anything.

This is precisely the property wanted from a translation between two languages' ASTs, or between a source AST and its lowered [[MLIR Operation|MLIR]]/[[LLVM IR]] form, or between two IRs claimed to be equivalent by an inserted proof (see [[Start Here]]): the translation should preserve whatever composition structure (e.g. "this statement follows that one," "this call depends on that definition") the source representation had. Framing translations-between-representations as functors gives a precise, checkable criterion for what "faithfully translated" means, rather than an informal one.

(In functional programming the word "functor" is also used more narrowly for a type constructor with a `map` operation — that usage is the same underlying concept, specialized to the category of types and functions in a particular language.)

## Related

- [[Category]]
- [[Category Theory]]
- [[Category Theory and Programming Languages]]
