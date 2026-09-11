"""
    Sophia

Julia side of the Sophia project: a content-addressed graph database of code,
its types, its proofs and its intermediate representations.

Design notes: `Sophia.md`, and `markdown/Design/Design Overview.md`.

COMMENTS ONLY — nothing in this package is implemented. Every `include` below
is commented out and every submodule file contains comments describing what it
is meant to do. Do not expect anything here to run.

# Why there is a Julia half at all

The split is in `markdown/Design/Repository Layout.md`, and it is not
arbitrary. Rust owns hashing, the core calculus, the kernel, the store and
code emission — hot loops over millions of small nodes, plus everything in the
trusted computing base. Julia owns the parts that only Julia can do:

  * **Introspection.** `Meta.lower`, `code_typed`, `CodeInfo`, method tables.
    There is no external Julia parser worth having, and there never will be;
    the only reliable Julia frontend is one that runs inside Julia.
  * **Annotation macros.** `@equiv`, `@spec`, `@sophia_test` have to expand in
    the user's own session.
  * **The precompilation experiment**, which hooks into Julia's own caching.
  * **Exploration.** The REPL is a better place to poke at a graph than a
    Rust test binary.

The boundary is a small C ABI: Rust exposes `extern "C"` entry points, Julia
calls them with `@ccall`. `CBinding` and `Clang_jll` are already dependencies
in `Project.toml`.

Worth stating plainly rather than hiding: **a project about eliminating FFI is
internally built on FFI.** That is fine. The FFI is an implementation detail of
the tool; it is not the mechanism by which user code is supposed to
interoperate.

# Intended submodules

| file             | role |
|------------------|------|
| `CoreIR.jl`      | Julia-side mirror of the Sophia Core term language |
| `Hashing.jl`     | hashing, and the conformance tests that keep it identical to Rust |
| `Store.jl`       | thin client over the Rust store |
| `Frontend.jl`    | Julia AST / lowered IR / typed IR -> core terms |
| `Annotations.jl` | `@equiv`, `@spec`, `@modulo`, `@sophia_test` |
| `Precompile.jl`  | the content-addressed code cache experiment |
"""
module Sophia

# include("CoreIR.jl")
# include("Hashing.jl")
# include("Store.jl")
# include("Frontend.jl")
# include("Annotations.jl")
# include("Precompile.jl")

# Intended top-level surface, once there is anything behind it:
#
#   Sophia.ingest(mod::Module)                  -> Vector{Hash}
#   Sophia.hash_of(f, argtypes)                 -> Hash
#   Sophia.context(h::Hash)                     -> a NamedTuple of everything
#                                                  attached to h
#   Sophia.@equiv f g modulo=(:alloc,)          -> records an EQUIV edge
#   Sophia.@spec f "postcondition"              -> records a Prop
#   Sophia.enable_cache!(store)                 -> route codegen through the
#                                                  content-addressed cache

end # module
