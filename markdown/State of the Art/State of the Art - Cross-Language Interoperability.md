# State of the Art - Cross-Language Interoperability

[[Start Here]] explicitly wants code from different languages to run together "with no use of FFI, just by means of parsing it into a graph database and inserting equivalency proofs." Existing approaches to multi-language interop, for contrast:

- **Foreign Function Interfaces (FFI)** — the default today (`ccall` in Julia, `extern "C"` in Rust/C++, JNI in Java, etc.). Requires each pair of languages to agree on a shared low-level calling convention and data layout; doesn't unify semantics, only lets one language call into compiled code from another as an opaque black box.
- **WebAssembly Component Model / WIT** — defines a language-neutral interface description format (WIT) and a shared, richer-than-C value model (variants, resources, strings) so components compiled from different source languages can call each other through a common ABI without hand-written bindings. Closer to the project's goal than raw FFI, but still interop *at compiled module boundaries*, not unification of the source-level semantics.
- **GraalVM / Truffle polyglot** — runs multiple languages (Java, JS, Python, Ruby, R, LLVM-bitcode-compiled languages) on one JVM-based runtime with a shared object model, allowing values to be passed between languages directly. The closest existing system to "multiple languages, one runtime, no FFI," though it unifies at the level of a shared runtime/object model rather than by proving source-level equivalence.
- **LLVM as a universal backend** — many languages already compile through [[LLVM IR]] (Rust, Swift, Julia, Clang-based C/C++, and more); this gives them a shared *target*, but they don't share a runtime, calling convention, or data representation without extra glue, and semantic differences (e.g. garbage collection strategy, memory model) don't disappear just because two languages both emit LLVM IR.
- **Schema-based interop (Protocol Buffers, Cap'n Proto, Thrift)** — define data shapes once in a neutral schema and generate per-language bindings; solves data interchange, not code/behavior sharing.

## Relevance

None of these attempt the specific claim [[Start Here]] wants to make — "this Julia code and this C++ code do the same thing," checked as a proof rather than assumed by a shared calling convention. The closest conceptual ancestor is really [[State of the Art - Proof-Carrying Code and Verified Compilation]] (proving properties about compiled code) combined with [[State of the Art - Equality Saturation and E-Graphs]] (representing many equivalent forms of "the same" computation) rather than anything in the interop space specifically.

## Related

- [[State of the Art - Proof-Carrying Code and Verified Compilation]]
- [[State of the Art - Equality Saturation and E-Graphs]]
- [[LLVM IR]]
