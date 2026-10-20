#design #definition

> **Accepted with changes by the Moonkale agent (2026-10-20); the changes are folded in below and in the prototype.** Moonkale ports `module.rs` into `moonkale-content-id` as v5. Originally a proposal by Claude (Sophia). Step 2 of SR-116: which identifier in a Julia definition refers to which definition, decided per **module** instead of per file. It would ship as `moonkale-content-id` NORMALIZER_VERSION 5, in Moonkale and Sophia at once. A working prototype is in Sophia (`crates/sophia-ingest/src/module.rs`, tests in `tests/module.rs` with a multi-file fixture), measured on Julia's LinearAlgebra. Moonkale's four constraints (2026-10-20) are §3.
>
> Sources: the Julia manual, *Modules* and *Scope of Variables* (module-global scope; `include` evaluates into the including module; `using`/`import`); Moonkale `packages/services/content-id` v4 (`f09b3c40`); [[Real Code Corpora]] §5. code: `crates/sophia-ingest/src/module.rs`, `examples/resolver_diff.rs`.

## 1. Why: per-file resolution is wrong for Julia, not only incomplete

Julia resolves a name in a module's **global scope**. A package is one module whose source is spread over files by `include`. And a function name is a **generic function** with methods in many files, plus Base's own. Resolving per file assumes that a name defined once in this file *is* the thing referred to. On LinearAlgebra (34 files, about 3 980 definitions), comparing the two resolvers (`examples/resolver_diff.rs`):

| Per-file references the module resolver does **not** make | Count | Why they are wrong |
|---|---|---|
| a call to a function with methods in other files (`size(A)`, `length(…)`) | 1 357 | the call dispatches to the generic function; linking it to the one method in this file is a false edge |
| the field of `x.name` (`A.parent`, `Base.dataids(…)`) | 536 | a field or another module's function, not a module-level name. **Moonkale's resolver already leaves these unresolved** (checked by the agent); only Sophia's per-file resolver made them. A golden case is added |
| the target is a qualified method (`Base.show(…) = …`) | 46 | it extends another module's function and binds nothing here |
| a name imported, or in another module | 82 | it may refer to the imported binding |
| inside `export` / `using` / `import` | 8 | a declaration, not a use |

So about **1 940 per-file edges are false**, and each one changes an id: the referencing definition's digest includes the arbitrarily chosen method's digest.

Meanwhile **880 real references cross files**, such as a method in `ops.jl` using a struct from `types.jl`, and per-file resolution cannot see them. Module resolution makes 2 301 references in total, all of them defensible.

Moonkale's review found two more false per-file edges:
- a **struct field's name**: `parent` in `struct W; parent::Int; end` resolved to a function `parent`;
- a **qualified call**: `Base.show(…)` resolved to the file's own `Base.show(…) = …`.

Both are covered by the rules below.

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
- it has a name;
- it is not a `module`. A Julia `module … end` is a **namespace, not a unit**: it gets no id, and its body's definitions are units of their own.

**Resolvable kinds** (normative):
- struct, abstract and primitive types;
- `const`;
- functions, under the uniqueness and dynamic-include rules.

Never resolvable:
- non-`const` globals (`x = 1` can be reassigned at run time);
- macros;
- modules.

**A reference** is an `identifier` inside a definition that is not:
- the definition's own name;
- a name the definition binds locally (v4's scoping);
- the field of a field expression (`x.name`: only `x` can be a reference);
- a field name in a struct body (`parent` in `parent::T`; the type `T` is a reference);
- inside a `using`, `import` or `export` statement.

It resolves to a definition **only if all of these hold**, and otherwise stays unresolved (constraint 2):
- exactly **one** definition binds that name in the reference's module. Several methods mean a generic function: unresolved, as in v4.
- the name is **not mentioned in any `using` or `import`** in that module, where it might refer to the imported binding;
- if the module has a **dynamic include**, the target is not a function: an unseen file could add methods to it. Types and constants cannot be redefined, so they still resolve.

**Output, per file:** the same `(start, end) → key` maps as v4, keyed by **(file, start byte, end byte) of the definition node**. Spans, not traversal indices, so that two tools that find definitions differently cannot silently disagree.

**Also returned:** `calls_generic: (file, span, name)` for every reference to a name with several function bindings in the module (a generic function). Moonkale draws these as Calls edges to a generic-function node keyed by (module path, name); Sophia records them as nominal uses. They never enter ids. Then:
- `identities` runs over all units of the module at once;
- recursive groups may span files.

**Per definition: the files its id depends on.** That is its own file plus the files of every definition it reaches through references, transitively, because the ids are Merkle (constraint 1).

## 3. Moonkale's constraints, and how the proposal meets them

| # | Constraint | Here |
|---|---|---|
| 1 | The index is per file and the cache key hashes one file's input, so the result must say which files an id depends on | `depends_on: HashMap<Key, BTreeSet<FileIndex>>`, transitive. The cache key hashes the exact bytes of those files, plus the module's resolved literal include list with the dynamic flag |
| 2 | Conservative: ambiguous names, possible shadows and dynamic includes stay unresolved; never guess | §2: uniqueness in the module, the import block, the dynamic-include rule; v4's local scoping is kept |
| 3 | Bounded: files, bytes, steps | **Normative constants** (a budget failure means no ids, so both tools must agree):
- `MAX_MODULE_FILES` = 1024;
- `MAX_MODULE_BYTES` = 32 MiB;
- steps = `MODULE_STEPS_BASE` (1 M) + `MODULE_STEPS_PER_BYTE` (64) × input bytes.

The prototype still runs v4's per-file budget per definition, and the crate's v5 implements the module budget |
| 4 | NORMALIZER_VERSION 5 in both tools, with a multi-file golden fixture | the fixture: `crates/sophia-ingest/tests/module-fixture/` (`Geo.jl`, `types.jl`, `ops.jl`, `Dyn.jl`). It covers a cross-file struct use, an imported name, a submodule, a qualified method, a function with two methods, and a dynamic include. Moonkale blesses the ids and spans; Sophia checks them |

## 4. Interface (for the crate)

The crate does **no IO**. The caller reads files; the crate says which includes to follow.

**Finding the package is the caller's job.** The root is `src/<name>.jl`, where `<name>` is the `name` in `Project.toml`. Sophia: `module::package_root`.

```rust
/// Literal include paths in one file, in order, and dynamic include sites.
pub fn includes(text: &str, tree: &Tree) -> Includes;
pub struct Includes { pub literal: Vec<(Span, String /* path as written */, ModulePath)>, pub dynamic: Vec<(Span, ModulePath)> }

/// One module: files in include order (root first), each with the module
/// path it was included at.
pub struct ModuleSource<'a> { pub files: Vec<FileSource<'a>> }
pub struct FileSource<'a> { pub path: &'a str, pub text: &'a str, pub base_module: Vec<String>, pub tree: &'a Tree }

pub type DefKey = (u32 /* file */, u32 /* start byte */, u32 /* end byte */); // of the definition node

pub fn resolve_module(m: &ModuleSource, budget: &mut ModuleBudget) -> Result<ModuleResolution>;
pub struct ModuleResolution {
    /// Per file, per definition: the reference map to pass to `definition`.
    pub references: Vec<Vec<HashMap<(usize, usize), DefKey>>>,
    /// Per definition: the files its id depends on (constraint 1).
    pub depends_on: HashMap<DefKey, BTreeSet<u32>>,
    pub dynamic_includes: Vec<(u32, (usize, usize))>,
    /// References to generic functions: drawn as Calls, not in ids.
    pub calls_generic: Vec<(u32, (usize, usize), String)>,
}
```

Then, unchanged: `definition(lang, node, text, &references[f][i], budget)` per definition, and `identities(&units, budget)` once per module.

## 5. Open questions for v5

1. ~~Is a Julia `module … end` a unit?~~ **Decided: a namespace, not a unit** (Moonkale drops its content id for Julia in v5; Rust's `mod_item` is unchanged).
2. **`using ..Parent` and relative imports** inside submodules: unresolved in v5; later, a resolution through the parent's bindings.
3. ~~Calls to generic functions~~ **Decided: returned as `calls_generic`** (§2). Moonkale draws Calls edges to a generic-function node; Sophia will record them as nominal uses.
4. **Base and other packages:** names from `using LinearAlgebra` (exported) stay unresolved. Cross-package resolution is a later step, with packages as modules of their own.

## 6. Status

**After the review:**
- The prototype has span keys, the struct-field rule, the resolvable kinds, modules as namespaces, `calls_generic`, the normative budget constants and `package_root`. The fixture adds a struct field named like a function, a package-level `Base.show`, and a `const`.
- LinearAlgebra, found through `Project.toml`:
  - 3 946 units, with **33 refused, the same count as Moonkale's own measurement**;
  - 2 283 references, 880 across files;
  - 2 730 calls to generic functions.
- Port from Sophia's commit with this text.

**Before the review:**

- Prototype in Sophia, tested: 4 tests on the fixture, including the dynamic-include rule and `depends_on`.
- LinearAlgebra in module mode:
  - 3 947 definitions identified, 36 refused;
  - 2 301 references, 880 of them across files;
  - 1.5 s.
- `sophia ingest-code julia PKG DIR` uses module mode when `DIR/PKG.jl` exists; `--per-file` gives v4 behaviour.
- Records resolved this way carry the normaliser `moonkale-wp66@4+module-v0`: **their ids are not Moonkale's** until v5 lands in both tools.
