#implementation

> Julia, Rust and Python through Moonkale's WP-66 normal form, from the shared crate `moonkale-content-id` (SR-116). Source: `code.rs`.
>
> Sources: Moonkale `packages/services/content-id` (rev `f09b3c40`, normaliser v4); [[Real Code Corpora]] rung 2. code: `code.rs`, `tests/code.rs`, `tests/golden-moonkale/`

- **Finding definitions:** `definitions` walks the tree with the crate's `is_definition`. Julia names come from the crate's `julia_name` (v6), the rule Moonkale uses too: callable-type and interpolated heads name nothing, and `Base.show` is qualified. Other languages use the `name` field.
- **Resolving references (`resolve`):** same file only, by spelling. It skips the definition's own name, the names it binds locally (parameters, assignment, `let` and loop targets), and names shared by several definitions (Julia methods: a call is to the generic function, so it is left unresolved, as Moonkale does).
- **Records (`file`):**
  - the id is the WP-66 digest, the same as Moonkale's `NodeId::from_content`;
  - `normaliser` is `moonkale-wp66@<version>`;
  - the bytes are the canonical parts, with references written as target digests;
  - `refs` are the targets outside the definition's recursive group.
  
  `Record::verify` does not apply: it knows only Sophia's own hashing rule, and a per-normaliser verifier is part of the language plug-in (SPEC-001 §3).
- **Refusals:** a definition the normal form refuses is skipped. References to it are dropped from the others, which are then normalised again, and the refusal is explained by the first refused construct inside it.

**Checked against Moonkale's golden fixtures:**
- with Moonkale's spans, all digests are equal in all three languages;
- with Sophia's resolver, no differences (since v3, which fixed the two Sophia reported: the struct self-edge, and Rust struct expressions).

**Modules and imports (2026-10-20).** A Julia `Def` also carries its `module` (the `module … end` blocks around it; in module mode, from the package root) and `extends`, the function a method adds to when it is not its own module's. `julia_scopes` collects every declared module and every `import M: f` / `import M.f`; `julia_extends` then places qualified heads and imported names with `locate`, which looks a module path up as Julia does: relative dots, then declared submodules innermost outwards, the enclosing module's own name, Base's exported submodules (`Broadcast`, `Iterators`, …), else a top-level module. **Approximation:** bindings that come from `using` and re-exports (`Base.Array` is `Core.Array`) are not followed; [[Julia Conformance]] measures what that costs.
