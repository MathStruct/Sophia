#implementation #julia #experiment

> The fixture of experiment E-5: one small package `Rules` in ten variants (`variants/<name>/`, each a package directory with `Project.toml` and `src/Rules.jl`), plus the Julia environment the oracle runs in (`Project.toml`, `Manifest.toml`: Zygote 0.7.18, ChainRulesCore 1.26.8, ChainRules 1.74.13, IRTools 0.4.25, SHA; resolved with Julia 1.12.7) and the oracle's output `truth.tsv` ([[derivatives]]).
>
> Sources: ChainRulesCore documentation (`rrule`, `@non_differentiable`); Zygote documentation (`@adjoint`). code: `variants/`, `truth.tsv`, `../derivatives.jl`, `../../derivatives.rs`

**`base`**: `sq(x) = x * x`; `g(x) = sin(x) * x` (plain code); `h(x) = exp(x) + one(x)` with a hand-written `ChainRulesCore.rrule` for it; callers `f1(x) = g(x) + sq(x)`, `f2(x) = g(sq(x))`, `f3(x) = sq(x) + one(x)`, `f4(x) = 2 * h(x)`; `unused(x) = 3 * x`, called by nothing; `p` and `q`, which call each other (one closure, gradients 1 and 2); `dyn(x) = Base.inferencebarrier(g)(x)`, a call Julia cannot infer. Gradients of `f1`–`f4`, `g`, `sq`, `h`, `p`, `q`, `dyn` at 0.7 and 0.7f0.

| variant | change | file |
|---|---|---|
| `rule_g` | an `rrule` for `g` in the package (counts its calls) | `src/added.jl` |
| `adjoint_g` | a rule for `g` in Zygote's own system, `Zygote.@adjoint` (methods of `ZygoteRules._pullback` and `ZygoteRules.adjoint`, made by a macro) | `src/added.jl` |
| `adjoint_g_other` | `adjoint_g` with another pullback (a probe: a wrong derivative on purpose) | `src/added.jl` |
| `nondiff_sq` | `ChainRulesCore.@non_differentiable sq(x)` (methods of `rrule` and `frule`, made by a macro) | `src/added.jl` |
| `method_g32` | a second method `g(x::Float32)` with another body | `src/added.jl` |
| `edit_unused` | `unused(x) = 4 * x` | `src/Rules.jl` |
| `pirate_g` | an `rrule` for `Rules.g` in another module of the session, outside the package | `outside.jl` |
| `pirate_g_other` | `pirate_g` with another pullback (a probe: a wrong derivative on purpose) | `outside.jl` |
| `pirate_sin` | an `rrule` for `Base.sin` at `Float64` outside the package, more specific than ChainRules's | `outside.jl` |

The `src/Rules.jl` of `rule_g`, `adjoint_g`, `adjoint_g_other`, `nondiff_sq` and `method_g32` is `base`'s with `include("added.jl")`; `pirate_g`, `pirate_g_other` and `pirate_sin` have `base`'s source unchanged. The rules are correct derivatives except the two probes, whose wrong gradients (0.7 for `g` at 0.7) make a program that missed a rule's body show as equal programs with different gradients ([[Review 2026-11-19 Codex B-026 (external)]], findings 1 and 2); `@non_differentiable` changes the numbers too (`sq`'s gradient is `nothing`).
