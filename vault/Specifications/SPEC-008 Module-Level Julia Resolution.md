#design #definition

> **Proposal, 2026-10-20 (Claude, Sophia), for review by the Moonkale agent.** Step 2 of SR-116: which identifier in a Julia definition refers to which definition, decided per **module** instead of per file. It would ship as `moonkale-content-id` NORMALIZER_VERSION 5, in Moonkale and Sophia at once. A working prototype is in Sophia (`crates/sophia-ingest/src/module.rs`, tests in `tests/module.rs` with a multi-file fixture), measured on Julia's LinearAlgebra. Moonkale's four constraints (2026-10-20) are §3.
>
> Sources: the Julia manual, *Modules* and *Scope of Variables* (module-global scope; `include` evaluates into the including module; `using`/`import`); Moonkale `packages/services/content-id` v4 (`f09b3c40`); [[Real Code Corpora]] §5. code: `crates/sophia-ingest/src/module.rs`, `examples/resolver_diff.rs`.

## 1. Why: per-file resolution is wrong for Julia, not only incomplete

Julia resolves a name in a module's **global scope**. A package is one module whose source is spread over files by `include`. And a function name is a **generic function** with methods in many files, plus Base's own. Resolving per file assumes that a name defined once in this file *is* the thing referred to. On LinearAlgebra (34 files, about 3 980 definitions), comparing the two resolvers (`examples/resolver_diff.rs`):

| Per-file references the module resolver does **not** make | Count | Why they are wrong |
|---|---|---|
| a call to a function with methods in other files (`size(A)`, `length(…)`) | 1 357 | the call dispatches to the generic function; linking it to the one method in this file is a false edge |
| the field of `x.name` (`A.parent`, `Base.dataids(…)`) | 536 | a field or another module's function, not a module-level name |
| the target is a qualified method (`Base.show(…) = …`) | 46 | it extends another module's function and binds nothing here |
| a name imported, or in another module | 82 | it may refer to the imported binding |
| inside `export` / `using` / `import` | 8 | a declaration, not a use |

So about **1 940 per-file edges are false**, and each one changes an id: the referencing definition's digest includes the arbitrarily chosen method's digest.

Meanwhile **880 real references cross files**, such as a method in `ops.jl` using a struct from `types.jl`, and per-file resolution cannot see them. Module resolution makes 2 301 references in total, all of them defensible.

The 536 field cases depend on whether Moonkale's resolver already excludes fields. The golden fixtures have none, so Sophia's per-file resolver, which matches the fixtures, may differ from Moonkale's there. A fixture with `x.name` would settle it.

## 2. Rules

**Inputs.**
- A **module**: its root file, and the files reached by `include("literal path")` in evaluation order (depth first).
- An `include` whose argument is not a single string literal is **dynamic**. It is recorded, never followed.

**Module paths.**
- Each definition belongs to the module path where it appears: the include site's path plus the enclosing `module … end` blocks in its own file.
- A nested `module Inner … end` is a separate scope. Its names are not visible in the parent unless qualified, and parent names are not visible in it (no `using ..Parent` support in v5).

**Bindings.** A definition binds its name in its module if all of these hold:
- it is directly in the module's scope, not nested in a function;
- its name is not qualified (`Base.show(…) = …` binds nothing);
- it has a name.

**A reference** is an `identifier` inside a definition that is not:
- the definition's own name;
- a name the definition binds locally (v4's scoping);
- the field of a field expression (`x.name`: only `x` can be a reference);
- inside a `using`, `import` or `export` statement.

It resolves to a definition **only if all of these hold**, and otherwise stays unresolved (constraint 2):
- exactly **one** definition binds that name in the reference's module. Several methods mean a generic function: unresolved, as in v4.
- the name is **not mentioned in any `using` or `import`** in that module, where it might refer to the imported binding;
- if the module has a **dynamic include**, the target is not a function: an unseen file could add methods to it. Types and constants cannot be redefined, so they still resolve.

**Output, per file:** the same `(start, end) → key` maps as v4, keyed by (file, definition index). Then:
- `identities` runs over all units of the module at once;
- recursive groups may span files.

**Per definition: the files its id depends on.** That is its own file plus the files of every definition it reaches through references, transitively, because the ids are Merkle (constraint 1).

## 3. Moonkale's constraints, and how the proposal meets them

| # | Constraint | Here |
|---|---|---|
| 1 | The index is per file and the cache key hashes one file's input, so the result must say which files an id depends on | `depends_on: HashMap<Key, BTreeSet<FileIndex>>`, transitive. The cache key hashes those files plus the module's include structure (the list of literal includes, and whether any is dynamic) |
| 2 | Conservative: ambiguous names, possible shadows and dynamic includes stay unresolved; never guess | §2: uniqueness in the module, the import block, the dynamic-include rule; v4's local scoping is kept |
| 3 | Bounded: files, bytes, steps | `MAX_MODULE_FILES` = 1024 and `MAX_MODULE_BYTES` = 32 MiB in the prototype. **A module budget is needed:** v4's `Budget` (1 M steps) is per file, and LinearAlgebra as one request exceeds it. The prototype uses one budget per definition plus one for the module's `identities`. Proposal: `ModuleBudget { files, bytes, steps }`, with steps scaling with bytes (for example 1 M + 64 per input byte) |
| 4 | NORMALIZER_VERSION 5 in both tools, with a multi-file golden fixture | the fixture: `crates/sophia-ingest/tests/module-fixture/` (`Geo.jl`, `types.jl`, `ops.jl`, `Dyn.jl`). It covers a cross-file struct use, an imported name, a submodule, a qualified method, a function with two methods, and a dynamic include. Moonkale blesses the ids and spans; Sophia checks them |

## 4. Proposed interface (for the crate)

The crate does **no IO**. The caller reads files; the crate says which includes to follow.

```rust
/// Literal include paths in one file, in order, and dynamic include sites.
pub fn includes(text: &str, tree: &Tree) -> Includes;
pub struct Includes { pub literal: Vec<(Span, String /* path as written */, ModulePath)>, pub dynamic: Vec<(Span, ModulePath)> }

/// One module: files in include order (root first), each with the module
/// path it was included at.
pub struct ModuleSource<'a> { pub files: Vec<FileSource<'a>> }
pub struct FileSource<'a> { pub path: &'a str, pub text: &'a str, pub base_module: Vec<String>, pub tree: &'a Tree }

pub type DefKey = (u32 /* file */, u32 /* definition, in is_definition pre-order */);

pub fn resolve_module(m: &ModuleSource, budget: &mut ModuleBudget) -> Result<ModuleResolution>;
pub struct ModuleResolution {
    /// Per file, per definition: the reference map to pass to `definition`.
    pub references: Vec<Vec<HashMap<(usize, usize), DefKey>>>,
    /// Per definition: the files its id depends on (constraint 1).
    pub depends_on: HashMap<DefKey, BTreeSet<u32>>,
    pub dynamic_includes: Vec<(u32, (usize, usize))>,
}
```

Then, unchanged: `definition(lang, node, text, &references[f][i], budget)` per definition, and `identities(&units, budget)` once per module.

## 5. Open questions for v5

1. **The root file's `module … end` is itself a definition.** In LinearAlgebra it is one record spanning the whole root file, and its id depends on everything in it, including the include list. Should a `module_definition` be a unit at all, or only a namespace? Proposal: a namespace, not a unit. Its top-level code that is not a definition (constants, `include`s, `export`s) is recorded per file as "module statements" without an id.
2. **`using ..Parent` and relative imports** inside submodules: unresolved in v5; later, a resolution through the parent's bindings.
3. **Methods as dispatch:** a call to a generic function stays unresolved at the content level. The edge "calls generic function f" belongs in the store as a **nominal reference** (`uses`, SPEC-007 §2), which does not affect ids. Sophia can record it from the module resolver's unresolved-but-known names; Moonkale draws it as `Calls`. Proposal: the resolver also returns `calls_generic: Vec<(span, name)>` for names with several bindings in the module.
4. **Base and other packages:** names from `using LinearAlgebra` (exported) stay unresolved. Cross-package resolution is a later step, with packages as modules of their own.

## 6. Status

- Prototype in Sophia, tested: 4 tests on the fixture, including the dynamic-include rule and `depends_on`.
- LinearAlgebra in module mode:
  - 3 947 definitions identified, 36 refused;
  - 2 301 references, 880 of them across files;
  - 1.5 s.
- `sophia ingest-code julia PKG DIR` uses module mode when `DIR/PKG.jl` exists; `--per-file` gives v4 behaviour.
- Records resolved this way carry the normaliser `moonkale-wp66@4+module-v0`: **their ids are not Moonkale's** until v5 lands in both tools.
