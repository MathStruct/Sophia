#comparison #reference

> Unison is the closest existing system: a language whose definitions are stored in a database by the hash of their syntax tree. Daniel asked to look at its specifications and adopt what applies (SR-70). This note goes through Unison's design point by point: **adopted**, **adopted with changes**, **later**, or **not adopted**, each with the reason.
>
> Sources: Unison language reference, *Hashes* (unison-lang.org/docs/language-reference/hashes); Unison codebase format v2 (SQLite) and Arya Irani's object-model design notes (hackmd.io/@aryairani/Sykk2jUo8); Chiusano and Bjarnason's talks; [[Unison]], [[Unison Codebase]], [[Unison Abilities]], [[Content-Addressed Code]] in this vault; Moonkale WP-18 (`packages/sources/unison/sources-unison.md`), which reads real Unison codebases. code: `crates/sophia-hash`, `crates/sophia-store`.

| # | Unison's design | Decision | Reason |
|---|---|---|---|
| 1 | Each definition is identified by the hash of its syntax tree, **excluding all names**. Names are metadata | **adopted** (SPEC-001 §4) | the core of the idea |
| 2 | **SHA3-512**, shown in base32hex, with **short hash prefixes** accepted when unambiguous | **adopted with changes**: BLAKE3-256; prefixes in display and input, as Unison does | 256 bits is enough, half the size, and much faster ([[Block Size and Hash Overhead]] §4) |
| 3 | **Cycles**: a mutually recursive group gets one hash; a member is `#x.n` | **adopted** (component digest plus index) | same structure |
| 4 | **Canonical order of a cycle**: "sorting all the members of the cycle by their individual hashes, with the cycle removed" | **adopted with changes**: Moonkale's WP-66 v1 order (the minimal serialisation over roots) | WP-66 is already implemented and reviewed on the Moonkale side, and sharing the algorithm keeps the two systems consistent. Both orderings are name-independent. To record when Sophia ingests Unison code: the orders differ, so Sophia ids for Unison code would be computed by Sophia's rule, and Unison's own hash kept as an attribute |
| 5 | **Constructors** are `#x.n#c`: the type's hash plus the constructor index | **adopted** (`Con(type, j)`) | |
| 6 | **Built-ins** have intrinsic names (`##Nat`), not digests | **adopted** (`Prim(name)` and built-in nominal types); confirmed by Moonkale's WP-18 | |
| 7 | **Structural vs `unique` types**: structural types are identified by their structure; `unique` types are generative | **adopted** (`data` vs `struct`/`abstract`, nominal entities in the store) | exactly the classification/identity split of [[Type Taxonomy]] §3 |
| 8 | **The codebase is a SQLite database** (v2 format). The stored form may differ from the canonical form that is hashed ("an object can be rewritten from one format to another without affecting its hash") | **adopted** (S-ADR-0005; canonical vs compact storage encoding, [[Block Size and Hash Overhead]] §2) | an independent confirmation of both choices |
| 9 | **Names in namespaces**, aliases allowed, rename only moves a name | **adopted** (SPEC-001 §5) | |
| 10 | **Namespaces with causal history** (a hash-linked history of namespace states), branches, projects | **later** (SPEC-001 §5.2) | v0 has one append-only log. The Merkle namespace snapshot is planned |
| 11 | **`update` propagates** a change to dependants that still typecheck, and leaves the rest as a `todo` list | **later** ([[SP-002 Propagating an update to dependants]]) | needs a type checker |
| 12 | **Scratch files**: you edit text in an ordinary file, and `ucm` adds or updates the definitions it contains | **adopted with changes**: the editor buffer of a projection is the scratch ([[S-ADR-0006 Edits through projections and the existing operations]]) | same idea, without a file on disk |
| 13 | **Typed documentation** (`Doc` values are Unison terms and can embed live code) | **adopted with changes**: docs are annotations, and a doc may embed references to definitions | docs must not change a definition's id; Unison links docs by a name convention, Sophia by annotation |
| 14 | **Abilities** (algebraic effects and handlers) | **later**, as sophialang's effect system ([[Computational Trinitarianism]] §4.4) | v0 core has no effects yet |
| 15 | **No type classes; no multiple dispatch** | **not adopted** | sophialang uses multiple dispatch (SR-59). That needs nominal generic functions ([[SP-006 Open method tables versus content addressing]]), which Unison avoids by not having open method tables |
| 16 | **Distributed computing by hash**: send a computation to another node, which fetches only the definitions it lacks (Unison Cloud) | **later**: Merkle sync of stores ("which of these hashes do you have?") | a natural consequence of content addressing |
| 17 | **Unison Share**: publishing and pulling namespaces | **later** | |
| 18 | **No dependent types; a Hindley–Milner-style type system with abilities** | **not adopted** | sophialang wants dependent types as an exchangeable checker (SR-53) |
| 19 | **No files, its own tooling for everything** | **adopted for sophialang only** | other languages keep their files (Moonkale ADR-0028 option B); Sophia stores their normal forms beside them |

**What Unison shows that matters most:** a content-addressed codebase is practical. Its rough edges were tooling (editor integration, version-control interop) and ecosystem size, not the identity scheme. Sophia's bet is that Moonkale fixes the first, and that ingesting existing languages (SR-03) fixes the second.

## Related

[[SPEC-001 Sophia Store v0]] · [[SPEC-003 Sophialang Core v0]] · [[State of the Art - Content-Addressable Code Systems]] · [[Moonkale Interface Needs]]
