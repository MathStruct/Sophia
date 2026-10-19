#implementation

> Crate root of `sophia-hash`: identity for the Sophia store, language-agnostic. Source: `sophia_hash.rs`.
>
> Sources: [[SPEC-001 Sophia Store v0]] §4; Moonkale Code Graph Schema, "WP-66 normalizer v1"; [[Adopting from Unison]]. code: `sophia_hash.rs`, [[canonical]], [[component]], [[merkle]], [[digest]]
>
> Theory (CT-ML wiki): [Abstract Syntax with Binding](https://mathstruct.org/CategoryTheory-ML-Wiki/Abstract-Syntax-with-Binding). This vault: [[Theory of the v0 Store]] §1.

## What it is for

Everything the store keys by content gets its identity here:
- a language's normaliser writes canonical member bytes with [[canonical]];
- [[component]] finds recursive groups and orders their members;
- [[merkle]] turns the ordered members into component and member digests, and defines `Record`, what the store keeps per definition.

The crate knows nothing about any language (S-ADR-0007).

## Why it is small and separate

Identity is the decision that cannot be cheaply revised: a change gives every definition in every store a new id, and the only remedy is a migration (SPEC-001 §4.5). So the crate has two dependencies (`blake3`, `thiserror`) and can be read in one sitting. It belongs to the [[Trusted Computing Base]]: if it is wrong, two different programs could share an id.

## Tests

14 unit tests: framing layout, nesting and truncation; dependency-first SCCs, including a 200 000-node chain (no recursion depth limit); order independence and automorphic members; the budget; digest parsing.
