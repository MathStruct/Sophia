#comparison #reference

> Would dedicated Lisp-machine-style hardware help Sophia? What would it require, and what would it look like (SR-69)? **Short answer:** the *compiled* sophialang program must not need it (SR-52 forbids inherent overhead, so ordinary CPUs are the target). The *autoresolved* levels (GC, dynamic dispatch, JIT) and the *store* (hashing, pointer chasing over a huge DAG) would benefit from a handful of features. Most of these now exist piecemeal in commodity hardware: tagged pointers, memory tagging, capabilities, hash acceleration through SIMD. A "Sophia machine" today would be a RISC-V core with CHERI capabilities, GC barrier assists and a dispatch cache, prototyped on an FPGA. Worth doing as an experiment late, not as a dependency.
>
> Sources: Moon, *Architecture of the Symbolics 3600*, ISCA 1985; Knight et al., *CADR*, MIT AI Memo 528, 1979; Ungar, *The Design and Evaluation of a High Performance Smalltalk System* (SOAR), 1987; SPARC V8 manual (`TADDcc`/`TSUBcc` tagged arithmetic); Click, Tene and Wolf, *The Pauseless GC Algorithm* (Azul Vega hardware read barriers), VEE 2005; Watson et al., *CHERI*, IEEE S&P 2015, and Arm Morello; Arm Memory Tagging Extension (MTE) and Top-Byte Ignore; Intel Linear Address Masking (LAM); O'Connor et al., *BLAKE3*, 2020; McGregor et al. on picoJava (bytecode hardware). code: none.

## 1. What Lisp hardware did, and why it lost

Lisp machines (MIT CADR, Symbolics 3600, LMI, TI Explorer) put the language's run-time checks into hardware:
- **Tagged words.** The 3600 used 36-bit words, with a type tag beside 32 bits of data. The ALU checked the tags *in parallel* with the operation: adding two fixnums cost one cycle, and a tag mismatch trapped to the slow path.
- **GC support.** Hardware assisted barriers and ephemeral (generational) collection.
- **Microcoded generic operations**, such as dispatch on tags and stack groups.
- **A single-address-space environment** (Genera).

They lost to commodity RISC CPUs in the late 1980s. Stock hardware improved faster than small custom-chip teams could follow, and compilers learned to do type checks in software (type inference, inline caches, tag bits in the low bits of pointers). The lesson: **specialised hardware wins only if its advantage is large and survives a few generations of commodity improvement.**

## 2. Where Sophia spends time

| Workload | Hot operations | Hardware that helps |
|---|---|---|
| **compiled sophialang, explicit or safe levels** | ordinary machine code: arithmetic, loads, stores | nothing special. This is the SR-52 promise |
| **autoresolve: GC** | allocation, read and write barriers, marking | hardware barriers (Azul Vega), memory tagging to cheapen barriers, a spare core (SR-56) |
| **autoresolve: dynamic dispatch** | type-tag reads, method-cache lookups | tagged pointers (TBI, LAM), a hardware dispatch cache (like the Smalltalk/SOAR method caches) |
| **autoresolve: JIT** | compiling, writing code, flushing the instruction cache | fast instruction-cache coherence; nothing exotic |
| **safe level: memory proofs** | none at run time (erased) | CHERI capabilities would let *unproved* unsafe code run with hardware-enforced bounds: a fourth level between unsafe and safe |
| **the store** | BLAKE3 hashing, B-tree lookups, pointer chasing through the DAG, deduplication | SIMD (BLAKE3 is fully vectorised: AVX-512, NEON); large caches; persistent memory for a single-level store; CXL-attached memory for huge stores |
| **queries and analytics** | joins, recursive traversal (Datalog) | columnar and vector units; graph accelerators are niche |

## 3. Commodity features that already cover most of it

- **Pointer tagging:**
  - Arm Top-Byte Ignore and Intel LAM let software keep tags in a pointer's upper bits for free;
  - Arm MTE checks a 4-bit tag on every access, which catches use-after-free and overflow in hardware;
  - SPARC had tagged add instructions in 1987.
- **Capabilities:** CHERI (Arm Morello boards, CHERI-RISC-V) replaces pointers with unforgeable, bounds-checked capabilities. This is CLOSOS's "pointer with access bits" in hardware ([[Operating Systems and Sophia]] §1). For Sophia it would make `Unsafe(memory)` sections that lack a proof *contained* rather than dangerous.
- **Hashing:** BLAKE3 reaches several GB/s per core on large inputs with AVX2 or AVX-512, and scales across cores. For Sophia's small inputs (one definition, a few hundred bytes), the per-call overhead dominates ([[Block Size and Hash Overhead]]). That is a software layout problem, not a hardware one.
- **Persistent memory:** Intel Optane DC is discontinued (2022), but CXL memory pools are the successor. A store mapped into CXL memory would be close to a single-level store.

## 4. What a "Sophia machine" would require

If one were built as an experiment, these would be the requirements:

1. **Capability-addressed memory** (CHERI), so that content ids map to capabilities and unsafe code is bounded.
2. **Tag-checked arithmetic and dispatch:** a small tag in each word or pointer, checked in parallel, plus a hardware **inline cache** keyed by (call site, argument type tags) → method entry. Most dynamic dispatch would then cost about what static dispatch costs.
3. **GC assist:** read and write barriers in hardware, done in parallel, with no extra instructions; concurrent marking on a dedicated small core with coherent access to the heap.
4. **A hash unit or wide SIMD** for BLAKE3 over small blocks at low latency, used by the store for every write.
5. **Memory as a cache of the store:** pages backed by content ids. A page fault fetches by hash, so deduplication is free. This is the System/38 single-level store, with hashes instead of addresses.

**What it would look like:** a RISC-V application core with the CHERI and tagging extensions, plus one or two small in-order cores for GC and hashing, sharing a coherent last-level cache. It would be prototyped on an FPGA (CHERI-RISC-V already runs on FPGAs), and the benchmark suite would compare it with stock x86 and Arm.

## 5. Assessment

- **Not needed** for the store, for the language or for compiled programs. Commodity CPUs are the target (SR-52).
- **Helpful** for the autoresolve levels (GC pauses, dispatch cost) and for safety (CHERI).
- **The one place hardware changes the design** is CHERI: it would add a level between unsafe and safe ("hardware-contained"), which may matter for running unproved code from many languages in one address space, the CLOSOS scenario.
- **As an experiment** (SR-73, "learn why it fails"): only after the GC and dispatch exist in software and are benchmarked, so that the hardware's gain can be measured rather than imagined.

## Related

[[Operating Systems and Sophia]] · [[Block Size and Hash Overhead]] · [[SPEC-002 Sophialang Principles]] · [[Benchmark Plan]]
