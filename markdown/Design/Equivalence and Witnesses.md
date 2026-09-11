# Equivalence and Witnesses

This is the novel claim of the project and the place where it can most easily go wrong. [[Start Here]] wants Julia code and C++ code to run together "with no use of FFI, just by means of parsing it into a graph database and inserting equivalency proofs". This note asks what such a proof could possibly say.

## First, the good news: most equivalence is free

If two definitions elaborate to the same [[Core Calculus|core term]], they get the same hash and are *literally the same node*. No edge, no proof, no checking. A surprising amount of cross-language sameness is of this kind — arithmetic on fixed-width integers, straight-line data manipulation, pure functions over primitives. Equivalence machinery is only needed where the core terms genuinely differ.

## The ladder of strength

An `EQUIV` edge is annotated with a level. The levels are totally ordered by how much they license:

| Level | Means | Decided by | Substitutable? |
| --- | --- | --- | --- |
| `alpha` | identical after canonicalisation | hash comparison | yes (trivially) |
| `defeq` | $Gamma ⊢ t ≡ u$ | kernel, via [[Normalization by Evaluation]] | yes |
| `rewrite` | connected by a chain of trusted rewrite rules | [[E-Graph|e-graph]] proof extraction | yes, if every rule is sound |
| `observational` | contextually equivalent w.r.t. a stated observation | a proof term in `Prf` | yes |
| `tested` | agree on a test suite / property fuzzing | executing [[Tests and Documentation as Nodes\|tests]] | **no** |
| `asserted` | a human said so | a signature | **no** |

The line between `observational` and `tested` is the line between a proof and evidence. Both belong in the database — evidence is genuinely useful — but only the top four may be used silently by the compiler to substitute one term for another. `tested` and `asserted` require an explicit policy opt-in, and any artifact built using them is tainted in its provenance record.

## What observational equivalence means

Within one language with one semantics, the standard definition is contextual equivalence:

$$ t ≃_"ctx" u quad ⟺ quad ∀ C[dot]. space (C[t] ⇓ v ⟺ C[u] ⇓ v) $$

quantified over all well-typed contexts `C`. This is the right notion and it is also famously impossible to prove directly, because of the quantification over all contexts. The standard workaround is a [[Logical Relations|logical relation]] — a type-indexed relation proved to be a congruence by construction, which implies contextual equivalence without quantifying over contexts. For stateful/effectful fragments the analogous tool is [[Bisimulation|applicative or environmental bisimulation]].

## What cross-language equivalence means

Here it gets harder, and the difficulty is usually skipped over in informal statements of this idea. Given two languages with semantics

$$ ⟦dot⟧_1 : "Term"_1 → cal(D)_1 quad "and" quad ⟦dot⟧_2 : "Term"_2 → cal(D)_2 $$

the sentence "`t_1` does the same thing as `t_2`" is **not well-formed** until you supply a correspondence between the two observation universes:

$$ R ⊆ cal(D)_1 × cal(D)_2 $$

and then the claim is $⟦t_1⟧_1 space R space ⟦t_2⟧_2$. `R` is not a technicality — it is the entire content of the claim. Concretely, for Julia ↔ C++ you must pin down:

- Julia `Int64` ↔ C++ `int64_t`: agree, except that signed overflow *wraps* in Julia and is *undefined behaviour* in C++. So `R` holds only on the non-overflowing subdomain, or the C++ side must be compiled with `-fwrapv`.
- Julia `String` (immutable, UTF-8, byte-indexed) ↔ `std::string` (mutable, byte sequence, no encoding guarantee): `R` must say which invariants are assumed.
- Julia `Array{T,N}` (column-major, 1-based, GC-owned, bounds-checked) ↔ `std::vector` / raw pointer (0-based, caller-owned, unchecked): `R` involves an ownership and layout story, not just a value story.
- Exceptions: Julia throws; C++ may throw, may `abort`, may be `noexcept`. Divergence and abnormal termination are *observations*.

This is precisely the setup of a **relational logical relation** between two languages, and it is precisely what multi-language semantics research ([[State of the Art - Language Semantics Frameworks]]) calls a *linking type* or *language interoperation semantics*. The honest framing: `R` is a piece of engineering that must be written once per language pair, reviewed carefully, and treated as part of the [[Trusted Computing Base]].

```tikz
\usepackage{tikz-cd}
\begin{document}
\begin{tikzcd}[column sep=huge, row sep=large]
t_1 \arrow[r, "\llbracket\cdot\rrbracket_1"] \arrow[d, dashed, "\approx"'] & \mathcal{D}_1 \arrow[d, "R", dashed] \\
t_2 \arrow[r, "\llbracket\cdot\rrbracket_2"'] & \mathcal{D}_2
\end{tikzcd}
\end{document}
```

The square must commute for the equivalence to hold. Sophia's actual strategy is to make the left column trivial by elaborating *both* sides into [[Core Calculus|SC]], so there is only one semantics and `R` reduces to a relation between SC representations — which is much more tractable, at the cost of pushing all the difficulty into the two frontends.

## Modulo: equivalence is never absolute

No useful equivalence relation on real programs ignores nothing. Every `EQUIV` edge carries a `modulo` set naming what the claim does not cover:

| Modulo tag | The claim ignores |
| --- | --- |
| `alloc` | number, size and timing of heap allocations |
| `timing` | wall-clock and asymptotic cost |
| `fp_assoc` | floating-point reassociation (so `(a+b)+c` vs `a+(b+c)`) |
| `fp_contract` | fusing multiply-add |
| `exception_identity` | *which* exception is thrown, only *that* one is |
| `gc_pressure` | effect on collector behaviour |
| `ordering` | order of observable effects that are claimed independent |
| `resource` | file handles, sockets, finalisation timing |

### The composition trap

Suppose $a approx_({"fp_assoc"}) b$ and $b approx_({"alloc"}) c$. Composing gives $a approx_({"fp_assoc", "alloc"}) c$ — the union. Chain a few of these and the modulo set grows until the claim says nothing. This is not a bug in the design, it is an accurate reflection of reality, but it means:

- `EQUIV` closure must be computed **lazily and per-query**, with the modulo budget supplied by the caller ("find me a replacement for `h` that is equivalent modulo at most `{alloc}`"). Eagerly materialising the transitive closure would be both huge and useless.
- The store should keep *shortest* witness chains, not just reachability.

### The congruence trap

For an equivalence to license substitution, it must be a **congruence**:

$ t approx u quad ⟹ quad C[t] approx C[u] quad ∀ C $

`defeq` and sound `rewrite` rules are congruences by construction. `asserted` and `tested` are emphatically not: two sorting functions that agree on every test may differ on stability, and a context that observes stability will distinguish them. This is why the substitutability column above is what it is, and it is the single most likely way for this system to silently produce wrong programs.

## Witness formats

A `Witness` node is a tagged union, and the checker for each is a separate, small, auditable program:

1. **Kernel derivation** — a `Prf`-fragment proof term of `t =_A u`, checked by the SC kernel. Strongest; needs someone to write it.
2. **Rewrite chain** — an ordered list of (rule hash, position, direction) steps; the checker replays them. This is what [[E-Graph|e-graph]] proof extraction produces, and it is the workhorse. Soundness reduces to soundness of the rule set, each rule being itself a witnessed node.
3. **SMT certificate** — a proof object from an SMT solver for a decidable fragment (bitvectors, linear arithmetic), plus the encoding used. Trust shifts to the encoder and the proof checker, not the solver.
4. **Translation-validation report** — the output of a per-instance equivalence checker on a specific lowering, in the style of [[State of the Art - Program Equivalence Checking|Alive2]]. This is the realistic mechanism for `Term → Op → Instr` edges, since proving MLIR and LLVM correct in general is not on the table.
5. **Test evidence** — suite hash, seed, inputs, results, environment. Evidence, not proof.
6. **Attestation** — an author signature over the claim. The weakest, and it must be attributable so it can be revoked.

## Refinement, the more useful sibling

Much of what one actually wants is not equality but **refinement**: `a` may be substituted for `b` because `a` is at least as defined and at least as deterministic.

$ a ⊑ b quad ⟺ quad ∀ C. space (C[b] ⇓ v ⟹ C[a] ⇓ v)  $

This handles the common asymmetric cases cleanly — a version with fewer allowed behaviours, a total implementation of a partial specification, a checked implementation of an unchecked one. `REFINES` is a separate edge kind because it does not compose with `EQUIV` in the same direction, and conflating them is a classic source of unsoundness.

## Related

- [[Cross-Language Semantic Hazards]] — the concrete failure list
- [[Contextual Equivalence]]
- [[Logical Relations]]
- [[Bisimulation]]
- [[Definitional vs Propositional Equality]]
- [[Trusted Computing Base]]
- [[State of the Art - Equality Saturation and E-Graphs]]
- [[State of the Art - Program Equivalence Checking]]
- [[sophia_equiv]] and [[witness]] — the Rust modules that would implement this
