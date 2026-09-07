# Under development

Our goal is a 

Todo:
try various graph databases:
- [ ] LadybugDB
- [ ] HelixDB
- [ ] TypeDB
and normal databases:
- [ ] duckdb
- [ ] Turso DB


Then take a simple language and parse the language statements into the graph database and build a compiler.
Simple Languages:
- [ ] metamath
- [ ] some Lisp variant

Then the ones I actually want: Julia, Lean, LLVM, MLIR, C++


We want to use extensive Hashing and normalization to give every statement in our database a uique  UUID. Ideally that UUID is uniquely determined by the syntax tree up to that statement (like in Unison)


Goal: this should contain multiple AST's and ideally build a compiler that queries the graph db and assembles IR's back into the graph database.
The compilation goal should be LLVM/MLIR as I do not want to get into the weeds of writing a compiler backend. 


WE were talking about that potentially we can create a new programming language that unifies much code under a single runtime. 
We will unify code by so called annotations: tHat means part of the statements in the graph database will not be there for compilation instead they prove equivalencies between codes.
I.e. that a piece of Julia code does the same than a piece of C++ code. 

It would be beneficial if code from other programming languages would just run together with other programming languages, with no use of FFI, just by means of parsing it into a graph database and inserting equaivalency proofs. 


If anyone ever disagrees with our implementation all he need to do is to rewrite the core of the graph compile and migrate the database to his own preffered schema.

This folder is also contains extensive markdown files which contains notes on what we did/figured out and some of the theory behind this. 
