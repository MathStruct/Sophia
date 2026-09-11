# sophia_hash

Crate root of `sophia-hash`. Source: `sophia_hash.rs` (comments only — nothing is implemented).

## What this crate is for

It computes the identity of everything in the system. A Sophia Core term goes in, a 256-bit [[BLAKE3]] digest comes out, and that digest is the primary key of every node in the store. The full scheme is in [[Hashing and Identity]]; this file is the public API around it.

## Why it is a separate crate with no workspace dependencies

Identity is the decision that cannot be cheaply revised. Changing the hash function, the encoding or the canonical form changes *every* node in *every* store, and the only remedy is a migration ([[Hashing and Identity]]). So the crate is deliberately small, dependency-free within the workspace, and sits at the bottom of the build order — it should be auditable in a sitting, because it is in the [[Trusted Computing Base]] and `sophia-store` deliberately is not.

## The three-hash API

| Function | Answers |
| --- | --- |
| `hash_term` → `h_def` | "is this the same definition?" |
| `hash_type` → `h_type` | "does this fit the same hole?" |
| `hash_erased` → `h_run` | "can I reuse the compiled code?" |

Collapsing them into one identifier is tempting and wrong in both directions: too fine and the [[Content-Addressed Precompilation|code cache]] never hits, too coarse and two different definitions become one node. Keeping three is cheap.

`hash_edge` exists because edges must be citable — a [[Equivalence and Witnesses|witness]] needs to refer to the specific `EQUIV` edge it justifies, and you cannot refer to something with no identity. See [[Graph Schema]].

## The encoding rules that carry the risk

Length-prefixed concatenation, fixed-width little-endian integers, IEEE bit patterns for float attributes (so `-0.0 ≠ +0.0`), and the schema version hashed first. Each of these is a one-line rule whose violation is a silent, global correctness failure rather than a crash. That asymmetry is why they are stated in the source file as well as here.

Domain separation via `blake3::derive_key` per node tag makes cross-kind collisions structurally impossible rather than improbable — cheap, so there is no reason not to.

## Submodules

- [[canonical]] — term → canonical form. The hard part, and the part in the TCB.
- [[merkle]] — canonical form → digest, including the [[Cycle Hashing|SCC construction]] for mutual recursion.

## Status

Comments only. The first thing that should actually be written, because [[Roadmap|M0]]'s acceptance test — re-ingesting an unchanged corpus writes zero new nodes — is a test of this crate and nothing else.

## Related

- [[Hashing and Identity]]
- [[Merkle DAG]] · [[BLAKE3]] · [[Hash Consing]] · [[UUID]]
- [[Trusted Computing Base]]
- [[Repository Layout]]
