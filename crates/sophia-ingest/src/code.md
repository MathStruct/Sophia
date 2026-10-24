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

**Moved to Moonkale (2026-10-24, SPEC-009 §8).** `Def`, `JuliaScopes`, `julia_scopes`, `julia_extends`, `locate` and `BASE_SUBMODULES` now live in `moonkale-content-id` (pinned at `c166fbd6`), ported from this file; `code::Def` is a re-export of its `Definition` (new fields `language`, `path_derived`, `diagnostic`). What stays here: finding definitions (`definitions`, `file`). Since `09ce6d9c` the crate also exports `modules_around` and `locate`, and Sophia's copy is gone; the crate has no depth limit any more either (SR-145, budgets on size and work). Re-ingesting LinearAlgebra on `09ce6d9c`: the same 905 names and 145 definition ids, all 3 152 records rebuilt identically, 1.45 s (2.48 s on `c166fbd6`). `tests/bindings.rs` checks that Sophia's definitions, named by the shared rule, are exactly the Julia rows of Moonkale's golden `bindings.tsv` (66 of 66). Rust and Python still use the per-file naming (`rs.<pkg>.<name>`); Moonkale's crate-and-module naming (SPEC-009 §7) is not switched on yet.
