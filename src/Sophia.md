# Sophia

Julia package entry point. Source: `Sophia.jl` (comments only; every `include` is commented out).

## Why there is a Julia half

The split is set out in [[Repository Layout]]. Rust owns hashing, the core calculus, the kernel, the store and emission — hot loops over millions of small nodes, plus everything in the [[Trusted Computing Base]]. Julia owns what only Julia can do:

- **Introspection.** `Meta.lower`, `code_typed`, `CodeInfo`, method tables. There is no external Julia parser worth having and there never will be; the only reliable Julia frontend runs *inside* Julia. See [[Julia Lowered IR]].
- **Annotation macros**, which must expand in the user's own session ([[Annotations]]).
- **The precompilation experiment**, which hooks into Julia's own cache machinery ([[Precompile]]).
- **Exploration** — the REPL beats a Rust test binary for poking at a graph.

## The FFI irony, stated rather than hidden

The Rust/Julia boundary is a small C ABI (`@ccall` over `extern "C"`; `CBinding` and `Clang_jll` are already in `Project.toml`). So **a project about eliminating FFI is internally built on FFI**.

That is not a contradiction worth being defensive about. The FFI is an implementation detail of the tool. The claim in [[Start Here]] is about how *user* code interoperates — through shared core terms and [[Equivalence and Witnesses|equivalence edges]] — not about how the compiler's own components talk to each other.

## Submodules

| File | Role |
| --- | --- |
| [[CoreIR]] | Julia-side mirror of the core term language |
| [[Hashing]] | hashing, plus the conformance tests that keep it byte-identical to Rust |
| [[Store]] | thin client over the Rust store |
| [[Frontend]] | Julia AST / lowered IR / typed IR → core terms |
| [[Annotations]] | `@equiv`, `@spec`, `@modulo`, `@sophia_test` |
| [[Precompile]] | the content-addressed code cache experiment |

## Related

- [[Design Overview]] · [[Repository Layout]]
- [[Content-Addressed Precompilation]] · [[Multiple Dispatch]] · [[World Age]]
