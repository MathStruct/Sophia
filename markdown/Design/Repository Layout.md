# Repository Layout

Sophia is simultaneously a Rust workspace, a Julia package and an Obsidian vault. This note fixes where things go and why, and is the index for the per-file design documents that sit next to the code.

## Constraints that force the shape

1. **Julia requires `src/<PackageName>.jl`.** Non-negotiable; `Project.toml` names the package `Sophia`, so `src/Sophia.jl` must exist and must be the package entry point.
2. **Cargo's defaults want `src/lib.rs` / `src/main.rs`,** but `[lib] path` and `[[bin]] path` are configurable, so Rust can move.
3. **The Obsidian vault root is the repository root** (`.obsidian/` lives there), so *every* markdown file in the repo is a vault note and `[[wikilinks]]` resolve across code and prose alike. This is deliberate: it is the filesystem-level version of the `DOCUMENTS` edge from [[Tests and Documentation as Nodes]].
4. **Wikilinks resolve by filename, not path,** so filenames must be globally unique in the repo. This is why no crate has a `lib.rs`.

(1) and (2) together mean Julia keeps `src/` and Rust moves to `crates/`.

## The tree

```
Sophia/
├── Cargo.toml                     # virtual workspace manifest
├── Project.toml                   # Julia package manifest
├── README.md
├── .obsidian/                     # vault config (inline-tikz, wypst)
│
├── markdown/                      # prose that is not attached to one file
│   ├── Start Here.md
│   ├── Design/                    # ← this folder
│   ├── Wiki/                      # background concept cards
│   └── State of the Art/          # survey of prior art
│
├── crates/                        # Rust: the performance-critical core
│   ├── sophia-hash/
│   │   └── src/{sophia_hash,canonical,merkle}.{rs,md}
│   ├── sophia-core/
│   │   └── src/{sophia_core,elaborate,effects}.{rs,md}
│   ├── sophia-store/
│   │   └── src/{sophia_store,schema}.{rs,md}
│   ├── sophia-equiv/
│   │   └── src/{sophia_equiv,egraph,witness}.{rs,md}
│   ├── sophia-emit/
│   │   └── src/sophia_emit.{rs,md}
│   ├── sophia-frontend-metamath/
│   │   └── src/metamath.{rs,md}
│   └── sophia-cli/
│       └── src/main.{rs,md}
│
└── src/                           # Julia: frontend, ergonomics, experiments
    ├── Sophia.jl / Sophia.md
    ├── CoreIR.jl / CoreIR.md
    ├── Hashing.jl / Hashing.md
    ├── Store.jl / Store.md
    ├── Frontend.jl / Frontend.md
    ├── Annotations.jl / Annotations.md
    └── Precompile.jl / Precompile.md
```

Every `.rs` and `.jl` file has a `.md` sibling of the same stem describing what that file is for. The code files contain **comments only** — this repository is at the design stage and nothing is meant to run yet.

## Why this split between Rust and Julia

Not arbitrary; each side does what it is actually better at.

| Concern | Side | Reason |
| --- | --- | --- |
| Canonicalisation, hashing | Rust | Hot loop over millions of small nodes; must be deterministic and allocation-disciplined |
| Core term representation, kernel | Rust | Same, plus it is in the [[Trusted Computing Base]] and wants to be small and auditable |
| Store access, query execution | Rust | Talks to DuckDB/FalkorDB via their C APIs |
| E-graph, saturation, extraction | Rust | `egg` already exists and is excellent ([[State of the Art - Equality Saturation and E-Graphs]]) |
| MLIR/LLVM emission | Rust | The C API bindings live there |
| **Julia frontend** | **Julia** | Only Julia can introspect Julia — `Meta.lower`, `code_typed`, `CodeInfo`, method tables. There is no external Julia parser worth having. |
| Annotation macros (`@equiv`, `@spec`) | Julia | They must run in the user's Julia session |
| Precompilation experiments | Julia | Hooks into Julia's own cache machinery |
| Exploratory analysis, plotting, REPL | Julia | It is the better environment for that |

The boundary is a C ABI: Rust exposes a small `extern "C"` surface, Julia calls it with `@ccall`. `CBinding` and `Clang_jll` are already dependencies in `Project.toml`, which suggests this is the intended direction. Note the irony, acknowledged rather than hidden: *a project about eliminating FFI is internally built on FFI.* That is fine — the FFI is an implementation detail of the tool, not the mechanism by which user code interoperates.

## Naming conventions

- Crates are `sophia-<area>` (kebab), their lib targets are `sophia_<area>` (snake).
- Each crate's lib target path is set explicitly so no file is named `lib.rs`:
  ```toml
  [lib]
  path = "src/sophia_hash.rs"
  ```
  This exists solely so wikilinks stay unambiguous — a rare case of documentation tooling dictating source layout, and a cheap price.
- Julia modules are `Sophia.<Name>`, one per file, included from `Sophia.jl`.
- Markdown siblings use the exact stem of their code file, so `[[canonical]]` links to `crates/sophia-hash/src/canonical.md`.

## Deliberately absent

- **No `build.rs` yet** — no native dependency is actually linked until there is something to link.
- **No tests** — there is no behaviour to test. Conformance suites arrive at [[Roadmap|M1]].
- **No `docs/` directory** — the vault is the documentation.
- **No C++ frontend crate** — see [[Cross-Language Semantic Hazards]] for why that is the hardest one, not the first one; when it comes it will be a thin Rust wrapper over a Clang plugin.

## Related

- [[Design Overview]]
- [[Roadmap]]
- [[Tests and Documentation as Nodes]]
- [[Sophia]] — the Julia package entry point
- [[main]] — the Rust CLI entry point
