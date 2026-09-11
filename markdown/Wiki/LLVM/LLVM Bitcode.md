# LLVM Bitcode

Bitcode is the dense binary serialization of [[LLVM IR]]. It's used to store compiled modules on disk or embed them in object files, to ship IR between separate compilation and link-time optimization (LTO) steps, and as the distribution format for things like precompiled libraries that still want to be optimized at final link time.

Bitcode is a serialization *format*, not a different representation — it round-trips to the same in-memory IR. It is relevant here mainly as one of the possible export targets once a graph database of code has assembled a function's IR: the graph is the source of truth, and bitcode (or textual `.ll`) is what gets emitted to hand off to the rest of the LLVM toolchain.

## Related

- [[LLVM IR]]
- [[LLVM]]
