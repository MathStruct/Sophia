#design #experiment

> Automatic differentiation as the first worked example of [[Storage and Materialization]]: a source program f, a transformation D, a derived program D(f), and a compiled artifact of D(f). Each is a different object with a different identity and a different case for being kept. Daniel and the external review both proposed AD for this (2026-10-20, SR-124). Measurements: E-AD-0 below; the experiment plan is [[Semantic Boundary Experiments]] E-5 to E-7.
>
> Sources: ChainRules.jl documentation (rules as methods of `rrule` and `frule`); Revels, Lubin and Papamarkou, *Forward-Mode Automatic Differentiation in Julia* (ForwardDiff), 2016; Innes, *Don't Unroll Adjoint: Differentiating SSA-Form Programs* (Zygote), 2018; Moses and Churavy, *Instead of Rewriting Foreign Code for Machine Learning, Automatically Synthesize Fast Gradients* (Enzyme), NeurIPS 2020; Mooncake.jl documentation; Reactant.jl documentation (tracing to MLIR/StableHLO, compilation through XLA); Lux.jl documentation (AD backends selected through ADTypes); [[SPEC-006 Worlds, Artifacts and Evidence]] §3, §6a; [[World Age]].

## The four objects

```text
   f                 source program            authored (category 1) or ingested
   D                 the transformation        a tool: name, version, mode, options
   D(f)              the derived program       category 5: derived; or 7: never exists as a program
   compile(D(f))     the compiled artifact     category 6: identity A (SPEC-006 §3)
```

Their identities must not be conflated:
- **f** has its content id C(f). Its derivative does not change it.
- **D(f)** is input-addressed:

  ```text
  id(D(f)) = H( D's name and version,
                mode and options        (forward/reverse, activity annotations, chunk size),
                C(f) and the ids of everything f calls (its closure),
                the rule world          (which rrule/frule methods exist: a method table, so a world),
                argument types          (D(f) is per signature in every Julia AD system) )
  ```

- **compile(D(f))** has A = H(id(D(f)), world, compiler, version, target, flags, …).

The **rule world** is the subtle input. ChainRules rules are methods of the generic functions `rrule` and `frule`. Adding an `rrule` for `g` changes D(f) for every f that calls g, **without changing f**. So a derivative depends on a method table, exactly as dispatch does ([[Julia Multiple Dispatch and World Age]]), and must cite the world it was built in.

## How Julia's AD systems map onto the objects

| System | How it differentiates | Does D(f) exist as a program? | What is expensive | What Sophia would keep |
|---|---|---|---|---|
| **ChainRules** (rules) | hand-written `rrule`/`frule` methods | the rule *is* authored source | nothing: it is written once | the rules as **authored** code (category 1), and their tests (ChainRulesTestUtils, finite differences) as **evidence** that the rule is right |
| **ForwardDiff** | operator overloading on `Dual` numbers | no: D(f) is f **specialised** at `Dual` types | Julia compiling that specialisation | nothing but a compiled artifact; D(f) is never a separate program (category 7 → 6) |
| **ReverseDiff** | records a tape at run time; can compile the tape | the tape (an intermediate) | taping per input structure | a compiled tape is the clearest **promoted intermediate** (category 7 → 6) |
| **Zygote** | source-to-source on Julia's lowered IR, in generated functions | yes, as IR, per signature | first-call compilation (well known to be slow) | the generated adjoint IR could be kept as a derived program; today Julia keeps it only in the session's method cache |
| **Enzyme** | differentiates **optimised LLVM IR** | yes, as LLVM IR | compilation | a derived program at the **LLVM layer**: the case for Sophia's multi-layer graph ([[Multi-AST Layering]]) |
| **Mooncake** | source-to-source on Julia IR, rule-based | yes, as IR | building the rule (`build_rrule`) | as Zygote; the built rule is an explicit, reusable object |
| **Reactant** | traces f into MLIR (StableHLO), differentiates with Enzyme-MLIR, compiles with XLA | yes, as an MLIR module | XLA compilation (seconds to minutes for real models) | the compiled executable, by A: expensive, deterministic per (shapes, types, flags), reused across training steps; the strongest case for persisting |

**Lux** chooses among these per call (`AutoZygote`, `AutoEnzyme`, `AutoForwardDiff`, `AutoMooncake`, and Reactant for compiled training), so a Lux model's gradient is not one object: it is one per backend, and two backends' D(f) are **different derived programs claimed to compute the same function**. That claim is evidence (category 4): tested numerically, never proved. Sophia should record it as a cross-artifact equivalence claim with `tested` trust (SPEC-006 §5, §6a).

## When to keep what

| Object | Keep? | Why |
|---|---|---|
| f | yes | authored |
| ChainRules rules | yes | authored; they are the human knowledge in AD |
| rule tests and their results | the latest result per (rule, world) | evidence a build may cite |
| D(f) as IR (Zygote, Mooncake, Enzyme) | **cache**; persist only if a measurement shows regeneration dominates | regeneration is a deterministic function of the inputs above, so a cache entry can always be rebuilt |
| compiled D(f), small functions | cache | E-AD-0: compiling is costly relative to one run, cheap in absolute terms |
| compiled D(f) of a model (Reactant/XLA) | **persist by A** | compile seconds to minutes, reused across thousands of steps and across sessions |
| tapes, traces, pass outputs | transient; promote a compiled tape when it is reused | category 7 |
| "backend X and Y agree on f's gradient" | yes, as tested evidence | the claim is the useful part; the runs are not |

## What this tests in Sophia's design

1. **Input addressing works for derived programs:** can id(D(f)) be computed **before** running D, from ids already in the store? If yes, the cache key exists statically and lookup precedes compilation. Failure modes: hidden inputs (global state, `@generated` code reading types at compile time, world-dependent rules not cited).
2. **World dependence of derived programs:** adding a rule must make exactly the D(g) that depend on it stale, and Sophia must find them by query (generic uses + rule world). Julia's own invalidation (backedges) is the comparison point.
3. **Layers:** Enzyme's D(f) lives at LLVM IR, Reactant's at MLIR. The derivative is a cross-layer object, which is what [[Multi-AST Layering]] is for.
4. **Equivalence across derived programs:** several backends, one mathematical derivative. A first real use of [[Equivalence and Witnesses]] at `tested` level.

## E-AD-0: measured cost of producing vs running D(f)

`experiments/ad-materialization/ad_costs.jl` ([[ad_costs]]): per AD system, warm the package on an unrelated function of the same argument type, then time the **first** gradient call of a fresh function (transformation + compilation of D(f)) and the **steady** call (best of 20 runs of the compiled D(f)), for x ∈ ℝ¹⁰⁰⁰ and three small functions. Julia 1.12.7, 2026-10-20.

| System | Produce D(f) (first call) | Run D(f) (steady) | Ratio |
|---|---|---|---|
| ForwardDiff | 173–211 ms | 0.15–1.4 ms | 150–1 200 |
| ReverseDiff | 338–398 ms (compiled tape: 69 ms) | 0.011–0.20 ms | 1 700–31 000 |
| Zygote | 225–516 ms | 0.0009–0.015 ms | 34 000–240 000 |
| Enzyme | 102–590 ms | 0.0004–0.016 ms | 11 000–1 260 000 |
| Mooncake | 0.8–6.6 s | 0.002–0.069 ms | 15 000–740 000 |
| Reactant | not measured: `Reactant_jll` does not resolve offline here | | |

**What it says.**
- **Producing a derivative costs 10² to 10⁶ runs of it.** For anything differentiated more than a few times, the derived, compiled program is worth keeping for the session, which Julia already does.
- **Across sessions Julia keeps nothing** of this for user code (pkgimages cache only what a package precompiles). Every new session pays 0.1 to 6.6 s **per function and signature** again. Persisting compiled D(f) by artifact id A is exactly what Sophia's artifact cache would add, and this is the case where it pays most clearly.
- **The derived program is expensive to make and probably cheap to keep**: an adjoint is IR of a size comparable to f's own (its size is not measured yet). A category 5 object with a high C(x) and a small S(x): the [[Materialization Policy]] inequality favours keeping it as soon as F(x) > 1.
- **The numbers mix two costs**: the AD transformation and Julia's compilation of the result. Separating them (Zygote's `@code_adjoint`, Mooncake's `build_rrule` vs first call) is the next step, because only the first is a pure function of Sophia-visible inputs.
- **Caveat:** three toy functions, one argument size, one machine. Enough to show the order of magnitude, not to model it.
