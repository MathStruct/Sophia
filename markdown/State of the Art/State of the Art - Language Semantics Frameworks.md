# State of the Art - Language Semantics Frameworks

If Sophia is to state that a Julia program and a C++ program mean the same thing, it needs a semantics for each. This note surveys the frameworks built for writing language semantics down formally — the infrastructure that a claim of cross-language equivalence would have to rest on.

- **K Framework** — the most ambitious. Languages are specified by rewrite rules over configurations, and from one specification K *derives* an interpreter, a symbolic execution engine, a deductive verifier and a model checker. Complete semantics exist for C (Ellison and Roșu's, which found real bugs in GCC test suites and in the standard itself), Java, JavaScript, Python, EVM (KEVM, used in production for smart-contract verification), x86-64 and LLVM IR. If a semantics for a Sophia frontend's source subset had to be written down and mechanised, K is the most plausible vehicle.
- **PLT Redex** — an executable DSL for [[Operational Semantics|reduction semantics]], with random testing of metatheory. Excellent for designing and debugging a calculus such as [[Core Calculus|SC]]; not intended for industrial-scale languages.
- **Ott** — a tool that takes an inference-rule specification and generates LaTeX *and* Coq/Isabelle/HOL definitions from one source. The obvious way to keep a written specification and a mechanised one in sync.
- **Spoofax / Statix** — a language workbench where syntax, name binding and type systems are declared once and tooling is generated. Its **scope graph** formalism for name resolution is directly relevant: it is a graph-based account of binding, which is precisely what a graph store of code needs.
- **Truffle / GraalVM** — not a semantics framework but a *self-optimising interpreter* framework: write an AST interpreter, get a JIT compiler via partial evaluation (Futamura projection). The pragmatic counterpoint — it delivers multi-language interop without anyone writing a formal semantics, and it works today ([[State of the Art - Cross-Language Interoperability]]).
- **RustBelt / Iris** — a semantic model of Rust's type system in a separation logic, used to verify that `unsafe` library code upholds the guarantees the safe interface promises. The state of the art in giving a real, complex, industrial language a usable semantics.
- **Lem**, **Sail** — specification languages for ISA and systems semantics; Sail is how ARM and RISC-V architectures are now formally specified. Evidence that even hardware vendors have converged on mechanised semantics.

## Multi-language semantics specifically

- **Matthews and Findler**, "Operational semantics for multi-language programs" — the foundational treatment, introducing *boundary* terms that mediate between two embedded languages.
- **Patterson and Ahmed**, linking types and compositional compiler correctness — how to state that compiled output from different source languages may be soundly linked. The theory Sophia's cross-language claims would instantiate.
- **Institutions** ([[Institution]]) and the Hets toolset — the specification-level treatment of translating between logics with meaning preserved.

## The uncomfortable conclusion for this project

**Julia has no formal semantics.** There is no K definition, no mechanised model, no paper defining `⇓` for Julia. There is a manual and an implementation. Some research exists on the type lattice and subtyping (Zappa Nardelli et al. formalised Julia's subtyping, which is a genuinely hard piece of work), but not on whole-language dynamic semantics.

The practical stance, recorded in [[Open Problems and Risks]]: do not attempt a semantics for Julia. Define the semantics of [[Core Calculus|SC]], make the Julia frontend handle an explicitly bounded subset, and treat the elaboration itself as the (unverified, differentially tested) specification of what that subset means. This is honest, it is achievable, and it is what every practical tool in this space actually does.

## Related

- [[Operational Semantics]]
- [[Core Calculus]]
- [[Institution]]
- [[State of the Art - Formal Semantics of Real Languages]]
- [[State of the Art - Cross-Language Interoperability]]
- [[Open Problems and Risks]]
