#overview

> sophialang, Sophia's own language: what exists, by level ([[Languages]]), and how other languages relate to it. 2026-10-20.
>
> Sources: [[SPEC-002 Sophialang Principles]]; [[SPEC-003 Sophialang Core v0]]; [[SPEC-005 Rendering Profiles]]; `crates/sophialang-core`, `crates/sophialang-syntax` ([[sophialang_core]], [[sophialang_syntax]]); [[Type Taxonomy]]; [[Computational Trinitarianism]].

## Status by level

| Level | Status | Evidence |
|---|---|---|
| L1 Parsed | **yes**: a surface syntax with two rendering profiles (Julia-like, Rust-like); parser with limits and diagnostics | round-trip tests: print → parse → same id, both profiles |
| L2 Normalised | **yes**: core terms with de Bruijn binders, canonical encoding, SCC hashing (`sophialang-core@0`) | `tests/identity.rs` (13): names never change ids, mutual recursion, holes hashed, decode/rehash |
| L3 Resolved | **yes**: elaboration resolves locals, same-text items, the store's names (innermost namespace outwards), built-ins, constructors | elaboration tests; edits through `sophia-interface` |
| L4 Lowered | it is the target IR: other languages would lower **into** it | — |
| L5 Executable | **no**: no evaluator, no type checker yet | — |
| L6 Verified | no | — |

## The core, briefly

- A dependently typed core with a `Prf`/`Cmp` split: propositions and proofs are separate from computations, and proofs never enter a computation's id (SPEC-002 §3).
- Definitions: functions, data types, relations (signature now, rules later), annotations; holes and unresolved names are allowed and hashed.
- Usages (linear, affine, unrestricted) and `Unsafe` marks are part of the content.
- Generic functions are nominal entities with versioned method tables, the same mechanism as Julia's in the store (SP-006).

## Relation to the other languages

- **Stored languages** (Julia, Rust, Python, Metamath) keep their own normal forms. Their records sit next to sophialang's in one store (S-ADR-0007).
- **Profiles are not languages.** The Julia profile prints sophialang in a Julia-like syntax; it is not Julia ([[Julia SophiaLang Interoperability]]).
- **Relations** between a foreign definition and a sophialang one (equivalence, refinement, lowering) are evidence records with a trust level (SPEC-006 §5, §6a). None exist yet.
- **The query language** is a separate language on shared primitives ([[Sophia Query Overview]], S-ADR-0012 proposed).

## Known limitations

- Nothing runs. "Lenticulum running in Sophia" (SR-115) needs L5 for sophialang **and** L4 for a Julia fragment, both far off.
- Two relations with the same signature and no rules are one definition (content addressing). Relations probably need nominal identity, like Julia types ([[Julia Types]]).
