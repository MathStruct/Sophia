# Unison

Unison is a statically-typed functional programming language whose defining feature is that definitions are identified and stored by a hash of their syntax tree, not by name or file path — see [[Content-Addressed Code]]. There are no source files to keep in sync with a build; the [[Unison Codebase]] is itself a kind of database of hash-identified definitions, and names are just mutable labels attached to hashes.

Unison also has a first-class effect system called [[Unison Abilities]] for describing and handling side effects (similar to algebraic effects), and Unison Cloud/distributed Unison can ship a single function's closure — code plus data — to run on a remote node, because a hash-identified definition is inherently self-contained and relocatable.

Unison is explicitly named in [[Start Here]] as prior art: "this idea somewhat has been done already by Unison." The key idea being borrowed is content-addressing — computing a unique, reproducible identifier for a piece of code from its own structure — not the rest of the language.

## Related

- [[Content-Addressed Code]]
- [[Unison Codebase]]
- [[Unison Abilities]]
- [[State of the Art - Content-Addressable Code Systems]]
