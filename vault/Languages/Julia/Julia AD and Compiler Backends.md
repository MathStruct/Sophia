#design #julia

> Julia's automatic differentiation systems and compiler backends (Zygote, Enzyme, Mooncake, ForwardDiff, ReverseDiff, Reactant), as Sophia would see them: programs that produce **derived programs** and **compiled artifacts** at different IR layers. Why Daniel prefers these to LuxCore as the next Julia target (SR-124). 2026-10-20.
>
> Sources: [[AD as a Materialization Case]] (the materialisation analysis and E-AD-0); the AD papers cited there; Lux.jl documentation (backends through ADTypes; Reactant for compiled training); Reactant.jl documentation (tracing, MLIR/StableHLO, XLA); [[Multi-AST Layering]]; [[MLIR]]; [[LLVM IR]].

## Why these are the hard part

Daniel, 2026-10-20: ingesting LuxCore "proves relatively little, as it avoids the AD and Reactant or other backend libraries that full Lux.jl brings with it." LuxCore is interfaces and small layer definitions. The difficult and interesting parts of the Julia ML stack are:
- **program transformations** that consume Julia code and produce other code (AD);
- **staging**: tracing Julia into another IR and compiling it elsewhere (Reactant → MLIR → XLA);
- **multiple layers**: Julia source, lowered IR, typed IR, LLVM IR, MLIR, each a place where a derived program can live;
- **rules as open method tables** (ChainRules), which make derived programs world-dependent.

## Where each system works

| System | Input layer | Output layer | Mechanism | Rules |
|---|---|---|---|---|
| ForwardDiff | Julia values (`Dual`) | Julia specialisation | operator overloading | its own `Dual` methods; ChainRules via extensions |
| ReverseDiff | Julia values (tracked) | a tape (optionally compiled) | operator overloading, taping | ChainRules via `@grad_from_chainrules` |
| Zygote | lowered/typed Julia IR | Julia IR (adjoints) | source-to-source in generated functions | ChainRules `rrule`s |
| Mooncake | Julia IR | Julia IR (rules) | source-to-source, explicit rule building | its own rules, can import ChainRules |
| Enzyme | optimised **LLVM IR** | LLVM IR | LLVM pass | EnzymeRules (`augmented_primal`, `reverse`) |
| Reactant | Julia code traced on `TracedRArray` | **MLIR** (StableHLO), compiled by XLA | tracing; AD by Enzyme-MLIR | traced operations |

## What Sophia would store

Following [[Object Categories]]:
- **authored:** the user's model code; ChainRules/EnzymeRules rules; Lux layer definitions;
- **evidence:** rule tests (finite differences); "backends X and Y give the same gradient on these inputs" claims;
- **derived programs:** adjoint IR (Zygote, Mooncake), differentiated LLVM IR (Enzyme), traced MLIR (Reactant); input-addressed by (system and version, mode, f's closure, rule world, argument types);
- **artifacts:** compiled code, XLA executables, by artifact id A;
- **transient:** tapes, traces, pass outputs.

## What Sophia needs to get there (and does not have)

| Need | Why | Status |
|---|---|---|
| Julia IR layers in the store (lowered, typed) | Zygote and Mooncake work there | not started; [[Julia Lowered IR]] is a background card |
| LLVM IR and MLIR as stored languages | Enzyme and Reactant output | not started; [[Multi-AST Layering]] is the design |
| a Julia process in the loop | every derived program comes from running Julia | not started; the conformance scripts are the first contact |
| world snapshots cited by derived programs | rules are methods | snapshots exist (SPEC-006 §2); citing them does not |
| transformation records | D(f) is a transformation's output | specified (SPEC-006 §6a), not built |

## Experiments

E-5 to E-7 in [[Semantic Boundary Experiments]]: rules versus derived derivatives (ChainRules and ForwardDiff/Zygote), derived programs at two layers (Zygote vs Enzyme), and staged compilation (Reactant). E-AD-0, the cost measurement, is done ([[AD as a Materialization Case]]).
