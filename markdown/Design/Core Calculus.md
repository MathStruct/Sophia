# Core Calculus

Every frontend elaborates into one common language, **Sophia Core (SC)**. Without a common language there is nothing to hash consistently, nothing to lower uniformly, and no shared vocabulary in which to state that a Julia function and a C++ function are [[Equivalence and Witnesses|equivalent]].

## Grammar

$ t, A ::= x | c | cal(U)_i | Pi (x : A). B | lambda x. t | t space u $
$ | Sigma (x : A). B | (t, u) | pi_1 t | pi_2 t $
$ | "data" D | "con"_k | "elim"_D $
$ | "let" x = t "in" u | mu x. t | "prim"_iota | t^epsilon $

Terms and types live in one syntactic category (types *are* terms), which is what [[Dependent Types|dependency]] requires. `μ` is general recursion, `prim` are machine primitives, and the superscript `ε` is an effect row ([[Effects Memory and Resources]]).

## Judgement forms

$ Gamma ⊢ t : A ! epsilon $

read "in context Γ, term `t` has type `A` and may perform effects `ε`". Two auxiliary judgements matter enormously for this project:

$ Gamma ⊢ A ≡ B quad ("definitional equality — decidable, silent") $
$ Gamma ⊢ p : A =_B C quad ("propositional equality — witnessed, stored") $

The first is decided by the kernel via [[Normalization by Evaluation]] and **is folded into the hash**: definitionally equal terms normalise to the same canonical form and therefore get the same identity. The second is a *proposition with a proof term*, and is exactly what an equivalence edge in the graph carries. The whole architecture hinges on this split — see [[Definitional vs Propositional Equality]].

## Selected rules

$ (Gamma ⊢ A : cal(U)_i quad Gamma, x : A ⊢ B : cal(U)_j) / (Gamma ⊢ Pi (x:A). B : cal(U)_(max(i,j))) $

$ (Gamma, x : A ⊢ t : B ! epsilon) / (Gamma ⊢ lambda x. t : Pi (x:A). B ! ∅) $

Note the effect on the abstraction is empty: building a closure performs no effects, *calling* it does. That distinction is the reason effects are on the judgement rather than on the type alone.

$ (Gamma ⊢ t : Pi (x:A).B ! epsilon_1 quad Gamma ⊢ u : A ! epsilon_2) / (Gamma ⊢ t space u : B[u slash x] ! epsilon_1 ∪ epsilon_2 ∪ "eff"(Pi)) $

$ (Gamma ⊢ t : A ! epsilon quad Gamma ⊢ A ≡ B) / (Gamma ⊢ t : B ! epsilon) quad ("conversion") $

The conversion rule is where a typechecker is forced to decide equality of arbitrary terms, and hence where the cost of dependent types actually lands.

## The consistency problem, and the two-fragment answer

A language with `μ` (general recursion) is **logically inconsistent** as a proof system: `μ x. x` inhabits every type, so every proposition is "provable". Julia code diverges, so SC must have `μ`. But witnesses in the graph must *mean* something, so the proof layer must be consistent. These requirements are irreconcilable in a single fragment.

The resolution is two fragments with a one-way door:

| Fragment | Recursion | Effects | Role |
| --- | --- | --- | --- |
| `Prf` | structural only (terminating) | none | witnesses, specifications, equivalence proofs |
| `Cmp` | general `μ` | arbitrary `ε` | actual programs, all frontends land here |

`Prf ↪ Cmp` is an inclusion (a proof can be run). There is **no** map `Cmp → Prf`; a general program cannot be treated as a proof. Statements *about* `Cmp` terms are expressible in `Prf` because `Cmp` terms are ordinary data there (quoted, by hash) — this is the standard "deep embedding" move, and it is what lets a total logic talk about a partial language.

```tikz
\usepackage{tikz-cd}
\begin{document}
\begin{tikzcd}[column sep=huge]
\mathsf{Prf} \arrow[r, hook, "\iota"] \arrow[dr, "\mathrm{quote}"'] & \mathsf{Cmp} \arrow[d, "\ulcorner\cdot\urcorner"] \\
 & \mathsf{Prf}\text{-}\mathrm{data}
\end{tikzcd}
\end{document}
```

## Machine primitives are part of the calculus, not an afterthought

Most core calculi treat integers as an opaque `Int`. That is fatal here: the entire premise is that a Julia definition and a C++ definition can be *proved* to agree, and they do not agree unless overflow, rounding and aliasing agree. So SC has explicit primitive types carrying their semantics:

- `i8 … i64`, `u8 … u64` with an explicit wrap/trap/poison discipline per operation (`add.wrap`, `add.nsw`, `add.trap`)
- `f32`, `f64` with IEEE-754 rounding mode and an explicit *contraction* and *reassociation* permission flag (there is no single "float addition")
- `ptr(r)` — a pointer into region `r`, never a bare integer
- `ref(r, A)` and `mutref(r, A)` — borrow forms, matching what [[Effects Memory and Resources]] needs

`prim` nodes therefore have a lot of attributes, and all of those attributes go into the hash. Two `add` nodes that differ only in overflow discipline are **different nodes**, which is correct: they are different functions. See [[Cross-Language Semantic Hazards]].

## What SC deliberately is not

- **Not a surface language.** Nobody writes SC. It is elaborated into, the way GHC Core or Lean's kernel language is.
- **Not total.** See above.
- **Not minimal for its own sake.** `Σ`, `let` and inductive families could all be encoded away, but encoding them away destroys the structural correspondence with source syntax that makes the hashes stable and the graph legible.
- **Not an optimisation IR.** Optimisation happens after lowering into [[MLIR Dialect|MLIR dialects]], where the existing infrastructure already lives.

## Related

- [[Multi-AST Layering]]
- [[Hashing and Identity]]
- [[Effects Memory and Resources]]
- [[Dependent Types]]
- [[Normalization by Evaluation]]
- [[Definitional vs Propositional Equality]]
- [[sophia_core]] — the Rust crate that would implement this
