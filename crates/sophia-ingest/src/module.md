#implementation

> Module-level Julia resolution: the prototype of the proposal [[SPEC-008 Module-Level Julia Resolution]] (step 2 of SR-116, for Moonkale's normaliser v5). Source: `module.rs`.
>
> Sources: [[SPEC-008 Module-Level Julia Resolution]]. code: `module.rs`, `tests/module.rs`, `tests/module-fixture/`, `examples/resolver_diff.rs`

- `discover(root)`: files in include order, following only `include("literal")`. Other includes are recorded as dynamic. Each file gets the module path it was included at. Bounded by `MAX_MODULE_FILES` and `MAX_MODULE_BYTES`.
- `resolve`: module-scope bindings (top level, unqualified, by module path; submodules are separate scopes). A reference resolves only to a unique binding that no `using` or `import` mentions; under a dynamic include, only to types and constants. Fields of `x.name`, and the contents of `using`, `import` and `export`, are never references. Also returns `depends_on`, the files each id depends on, transitively.
- `ingest`: identities over the whole module (recursive groups may span files), with refusals handled as in `code::file`. Records carry the normaliser `moonkale-wp66@4+module-v0`, because these ids are not Moonkale's until v5.
- **After the Moonkale review:**
  - keys are spans (`(file, start, end)`);
  - struct field names are not references;
  - only types, consts and functions resolve;
  - Julia modules are namespaces;
  - `calls_generic` is returned;
  - the budget constants (`MODULE_STEPS_*`) are normative;
  - `package_root` finds `src/<name>.jl` from `Project.toml`.
- v4's `Budget` is per file. The prototype uses one budget per definition and one for the module's `identities`; SPEC-008 §3 proposes a module budget.
