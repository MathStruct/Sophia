#design #julia

> How Julia source becomes records with content ids: the parser, the WP-66 normal form shared with Moonkale, and what it refuses. Levels L1 and L2 of [[Languages]]. 2026-10-20.
>
> Sources: Moonkale `packages/services/content-id` (`lib.rs`, `julia.rs`; normaliser v6, rev `31f0f0d4`); [[SPEC-008 Module-Level Julia Resolution]]; [[Real Code Corpora]] §5; `crates/sophia-ingest/src/code.rs`, `module.rs`.

## Parser

- **tree-sitter-julia**, through `arborium` (Moonkale's grammar bundle). Both tools parse with the same grammar version, pinned by the crate's revision.
- tree-sitter is error-tolerant: a file with a syntax error still parses, with `ERROR` nodes. A definition containing one is refused (`MalformedSyntax`).
- **Limits** are part of the normal form: at most 1 024 files and 32 MiB per module, and a step budget of 10⁶ + 64 per source byte. Over them, no ids at all, rather than different ids.
- **Not Julia's parser.** JuliaSyntax.jl is the reference. tree-sitter-julia agrees on ordinary code; disagreements on rare syntax (juxtaposition, some string macros, `where` precedence corners) are possible and would show up as refusals or as differences against JuliaSyntax. Checking against JuliaSyntax is an open conformance item ([[Julia Conformance]]).

## What a unit is

A **unit** is a definition directly in a module's scope: a function (long or short form, with `where`), a struct, an abstract or primitive type, a constant or global assignment, a macro. A module is a namespace, not a unit. Nested functions are part of their enclosing unit.

**Known wrong unit:** a keyword default inside a macro call at module level (`@deprecate f(x; check::Bool = true) g(x)`) is taken as an assignment unit named `check` (11 cases in LinearAlgebra; reported to Moonkale).

## The normal form (WP-66 v6)

A unit's canonical form is a sequence of **parts**: bytes (the normalised syntax) and references (to other units). What the encoding does:
- drops comments and formatting; docstrings are not part of the content (they are documentation, SP-001);
- **locals are positional**: parameters, `let` and loop variables, destructuring targets, type parameters (`where T`) are numbered by binding order in their scope, so renaming a local does not change the id;
- the unit's **own name is not content**: two identical bodies under different names are one record (134 such shared bodies in LinearAlgebra under v2);
- a module-level name the resolver bound to exactly one unit becomes a **reference** to that unit's id; recursive groups are hashed together (SCCs, canonical order);
- any other free name (a Base function, a generic function of the module, a type from another package) stays **as written**: spelling, not identity.

The id is H(language, normaliser version, parts with references replaced by target ids). Version v6 is recorded on each record as `moonkale-wp66@6`.

## What is refused

LinearAlgebra under v6: 136 of 3 571 definitions.

| Reason | Count | What |
|---|---|---|
| `DependencyRefused` | 103 | refers to a refused unit, so its id cannot be computed |
| `UnsupportedSyntax` | 27 | constructs whose scoping the normal form does not model yet (e.g. some nested named functions) |
| `MalformedSyntax` | 6 | parse errors, mostly tree-sitter disagreeing with Julia on rare syntax |

Earlier versions refused far more (v2: 31 %, for loops, varargs and destructuring), which is the history in [[Real Code Corpora]] §5.

## The cost of keeping the canonical form

The parts encoding is about **9× the source** (12 MB for LinearAlgebra's 1.34 MB). It is a fine hashing input and a poor storage format; the source is the authority ([[Current Inventory]]).
