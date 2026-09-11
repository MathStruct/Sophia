# main

The `sophia` command-line driver. Source: `main.rs` (comments only, plus an empty `fn main` so the workspace still builds).

This file is where cargo's hello-world lived before the workspace restructure described in [[Repository Layout]].

## What the CLI is for

It is how the graph becomes legible to a human before any editor integration exists. Every subcommand corresponds to a query in [[Query Cookbook]] — the CLI is a thin shell over the store, which is the right shape for a system whose thesis is that a compiler is a database.

## The two subcommands that are the actual demos

**`sophia context <hash>`** is the capability [[Start Here]] predicts: "one could query the exact context of a piece of code". It returns the definition, its type, its tests, its docs, its callers and callees, every known alternative implementation with the strength of each claim, and its source span. Assembling that today takes an IDE, a test runner, a coverage tool and a documentation generator — and still misses the alternatives, because nothing records them.

It is the most convincing thing to show someone early, and it depends on none of the risky machinery: just ingestion, hashing and storage. See [[Tests and Documentation as Nodes]].

**`sophia trust <artifact>`** is the honesty feature. It prints the weakest [[Equivalence and Witnesses|equivalence level]] anything in the derivation relied on, plus the union of every `modulo` tag along the way. A system that cannot answer this question should not be trusted to substitute one piece of code for another; being able to answer it is what makes the weaker evidence levels safe to have in the database at all. See [[Trusted Computing Base]].

## Output discipline

Human-readable by default, `--json` for everything, hashes abbreviated to 160 bits for reading with `--full` available. Truncation is for reading, never for keys ([[UUID]]).

## What deliberately is not here

No editing — source stays in files and Git for the whole early roadmap ([[Naming and Change Propagation]]), because a Unison-style codebase manager fights every existing editor, reviewer and CI system. No daemon, no server, no LSP. And no convenience flag that silently accepts `Asserted`-level equivalences: if a build needs one, it must say so in its output *and* in the artifact's provenance.

## Related

- [[Query Cookbook]] · [[Repository Layout]]
- [[Tests and Documentation as Nodes]] · [[Trusted Computing Base]]
- [[sophia_store]] · [[sophia_equiv]] · [[sophia_emit]]
