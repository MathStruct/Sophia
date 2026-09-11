The idea was to store statements from programming languages and all their intermediate representations down to LLVM and MLIR in a graph data base and build a compiler around the graph database.
This idea somewhat has been done already by [Unison](https://www.unison-lang.org/)
However we have a few goals, where we do not know whether they are viable:
- New programming language means new ecosystem --> We wanna avoid that and make old software usable. It would be beneficial if code from other programming languages would just run together with other programming languages, with no use of FFI, just by means of parsing it into a graph database and inserting equivalency proofs. 
- There is the Curry-Howard-Lambeck correspondence. We would like to build multiple ASTs, for types, functions variables. 
- I do not assume that this becomes clean ASTs. However we would like to make extensive use of hashing and calculate unique reproducible UUIDs from the tree of dependencies. and/or insert equivalency proofs for two distinct UUIDs.

We were talking about that potentially we can create a new programming language that unifies much code under a single runtime. 
We will unify code by so called annotations: that means part of the statements in the graph database will not be there for compilation instead they prove equivalencies between codes.
I.e. that a piece of Julia code does the same than a piece of C++ code. or correctness or solve memory management. Or just contain comments, markdown, etc. 

Also this would be very beneficial for as one could query the exact context of a piece of code in the database.  

Tests would be inserted as nodes on a piece of code. 


The databases I considered where LadybugDB, HelixDB, TypeDB or FalkorDB as well as DuckDB and TursoDB.

If anyone ever disagrees with our implementation all he need to do is to rewrite the core of the graph compile and migrate the database to his own preffered schema.

Actually I originally wanted Julia + Lean combined i.e. a Julia with dependent types. Also I wanted to improve precompilation for Julia. This I wanted to solve with a graph database.

## See also

- [[Design Overview]] — what the above becomes once it is made precise: the core calculus, the hashing scheme, the graph schema, the equivalence ladder, and an honest risk list
- [[Wiki Home]] — background concept cards (LLVM, MLIR, Unison, hashing, rewriting, semantics, type theory, category theory)
- [[State of the Art]] — survey of existing systems and research related to these goals
- [[Repository Layout]] — where the code and its per-file design notes live
- [[Glossary]] — project vocabulary, including the terms this vault uses differently from the text above

## Reader's note on vocabulary

This page says "UUID" for what the rest of the vault calls a **content hash**; see [[Hashing and Identity]] and [[UUID]]. It says "equivalency proofs" for a family of claims that turn out to differ enormously in strength, from "identical after normalisation" to "a human asserted it"; that ladder is in [[Equivalence and Witnesses]]. And "multiple ASTs for types, functions, variables" resolves into two independent axes rather than three trees — see [[Multi-AST Layering]].