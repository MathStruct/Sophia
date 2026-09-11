# State of the Art - IR Interchange Formats

Sophia's graph is, among other things, a *format* for exchanging code between systems. Many such formats already exist; each embodies a decision about how much semantics to carry, and the pattern across them is informative.

| Format | Level | Carries types? | Designed for |
| --- | --- | --- | --- |
| **[[LLVM Bitcode]]** | low, SSA | yes, low-level | LTO, shipping IR between compiler stages |
| **WebAssembly** | low, stack machine | yes, simple | sandboxed portable execution; formally specified |
| **Wasm Component Model / WIT** | interface | rich (variants, resources, strings) | cross-language composition without an FFI |
| **SPIR-V** | mid, SSA | yes, GPU-oriented | portable GPU shaders/kernels; Khronos-standardised |
| **JVM bytecode** | mid, stack machine | yes, nominal | portable managed execution |
| **.NET CIL** | mid, stack machine | yes, incl. generics | multi-language runtime, by design |
| **Cranelift CLIF** | low, SSA | yes | a fast, simple backend IR |
| **[[MLIR]] bytecode** | *any*, extensible | yes, dialect-defined | multi-level IR interchange |
| **StableHLO / ONNX / TOSA** | high, tensor ops | yes, tensors | ML model interchange |
| **TVM Relay/TensorIR, XLA HLO** | high | yes | ML compiler frontends |
| **Unison's serialised terms** | high, typed λ | yes, dependent-ish | [[Content-Addressed Code\|content-addressed]] code exchange |

## What the pattern shows

1. **.NET CIL is the closest existing thing to [[Start Here]]'s goal** — a common IR that several languages (C#, F#, VB) compile to and genuinely interoperate through, with a shared type system, shared object model and one garbage collector. The instructive part is what it required: the languages had to *agree* on the Common Type System, and languages that would not agree (C++, dynamic languages) got second-class treatment (C++/CLI, the DLR) and never really fit. **Unification through a common IR works exactly to the extent that the languages were willing to converge.** That is the single most useful historical lesson available to this project.
2. **The Wasm Component Model is the current best answer to the no-FFI goal** and deserves close study — it is a serious, funded, standardised attempt at the same problem, solved by defining a rich interface language and generating the glue, rather than by proving equivalences. Sophia should be able to say precisely how it differs, and the difference is: WIT unifies *interfaces*, Sophia wants to unify *implementations*.
3. **MLIR is the only format designed to be extensible across abstraction levels**, which is why it remains the right lowering target ([[MLIR vs LLVM IR]]).
4. **Nobody's interchange format carries proofs.** All of them carry types; none carries evidence that two modules are equivalent. That gap is where Sophia is genuinely new, and also where it has no prior art to copy.

## Design implications

- The store's serialisation should be **layer-tagged** and extensible in the MLIR style, not a fixed instruction set.
- A **WIT-like interface layer** may be the pragmatic escape hatch when equivalence cannot be proved: fall back on a declared interface and generated glue, and record that the boundary is unproven. Better a working system with labelled trust boundaries than a stalled one.
- The CIL lesson argues for **being explicit about the common type system** early ([[Core Calculus]]), and for accepting that languages which will not fit it get partial support rather than pretending otherwise.

## Related

- [[State of the Art - Cross-Language Interoperability]]
- [[MLIR vs LLVM IR]]
- [[LLVM Bitcode]]
- [[Core Calculus]]
- [[Content-Addressed Code]]
