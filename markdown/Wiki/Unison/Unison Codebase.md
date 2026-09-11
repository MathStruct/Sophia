# Unison Codebase

In Unison, "the codebase" is not a tree of source files but a database: every definition is stored once, keyed by its content hash (see [[Content-Addressed Code]]), along with a separate table of names that point at those hashes. Editing code means writing a new definition (which gets a new hash) and updating which name points to it — old versions aren't destructively overwritten, they're just no longer pointed to by the name in question, so history is naturally preserved.

Tooling (the Unison Codebase Manager / UCM) mediates all edits; there's no text-file editing-and-saving step that a compiler then re-parses from scratch on every build, because a definition that already exists by hash never needs to be re-parsed or re-typechecked.

This is close to the storage model implied by [[Start Here]]: a graph database of statements/definitions, addressed by hash, with names, types, tests, and equivalency proofs attached as additional nodes/edges rather than baked into the identity of the code itself.

## Related

- [[Unison]]
- [[Content-Addressed Code]]
