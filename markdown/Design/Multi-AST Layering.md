# Multi-AST Layering

[[Start Here]] says: "There is the Curry-Howard-Lambek correspondence. We would like to build multiple ASTs, for types, functions, variables." This note takes that seriously and asks what the multiple ASTs actually are, and how they relate.

## Two different axes are being conflated

The phrase "multiple ASTs" covers two independent things, and separating them clarifies a lot:

**Axis 1 — the correspondence axis (terms / types / proofs).** These are *not* three separate trees. Under [[Curry-Howard-Lambeck Correspondence|Curry-Howard-Lambek]] they are one syntactic category viewed three ways: a type is a term, a proof is a term, a program is a term. In [[Core Calculus|SC]] they are literally the same node kind. What differs is the *fragment* they live in (`Prf` vs `Cmp`) and the *role edge* that connects them (`HAS_TYPE`, `PROVES`). So the design answer is: one AST, three roles, distinguished by edges rather than by node type.

**Axis 2 — the abstraction axis (source / core / MLIR / LLVM).** These genuinely *are* different trees with different node vocabularies, connected by `ELABORATES_TO` and `LOWERS_TO`. This is the [[MLIR Lowering|progressive lowering]] idea, persisted.

Confusing the two leads to the mistake of putting types in a separate database table from terms, which immediately breaks dependent types (where a type contains a term).

## The abstraction axis, as a diagram

```tikz
\usepackage{tikz-cd}
\begin{document}
\begin{tikzcd}[column sep=large, row sep=large]
\mathcal{L}_{\mathrm{Julia}} \arrow[dr, "F_J"'] & \mathcal{L}_{\mathrm{C{+}{+}}} \arrow[d, "F_C"] & \mathcal{L}_{\mathrm{Lean}} \arrow[dl, "F_L"] \\
 & \mathcal{C} \arrow[d, "G"] & \\
 & \mathcal{M} \arrow[d, "H"] & \\
 & \mathcal{V} &
\end{tikzcd}
\end{document}
```

Each of `L_Julia`, `L_C++`, `L_Lean` is the category whose objects are that language's programs and whose morphisms are its context-formation/substitution structure. `C` is [[Core Calculus|SC]], `M` is [[MLIR]], `V` is [[LLVM IR]].

The claim implicit in the design is that `F_J`, `F_C`, `F_L`, `G`, `H` are [[Functor|functors]] — structure-preserving. That is a *falsifiable, useful* claim, not decoration:

- **Preserves composition**: if `g ∘ f` is expressible in the source, the elaboration of the composite is the composite of the elaborations. Any frontend that special-cases composite expressions (e.g. a peephole in the parser) breaks this and will produce hashes that do not compose.
- **Preserves identities**: an identity/no-op in the source elaborates to an identity in core. Sounds trivial; fails the moment a frontend inserts a coercion.

## Cross-language equivalence as a natural transformation

If a Julia program and a C++ program compute the same thing, the relationship is not just a pair of unrelated elaborations — it should hold *uniformly*, for the whole family of related programs, and commute with composition. That is exactly a [[Natural Transformation|natural transformation]].

Concretely, suppose `P` is a shared "protocol" category (say, the signatures of a numerical library) with interpretations `A : P → C` (via Julia) and `B : P → C` (via C++). A family of equivalences $eta_X : A(X) → B(X)$ is natural iff for every morphism $f : X → Y$ in `P`:

```tikz
\usepackage{tikz-cd}
\begin{document}
\begin{tikzcd}[column sep=huge, row sep=large]
A(X) \arrow[r, "\eta_X"] \arrow[d, "A(f)"'] & B(X) \arrow[d, "B(f)"] \\
A(Y) \arrow[r, "\eta_Y"'] & B(Y)
\end{tikzcd}
\end{document}
```

commutes. In plain terms: *if you claim `jl_sort ≈ cpp_sort` and `jl_map ≈ cpp_map`, naturality is the demand that `jl_map ∘ jl_sort ≈ cpp_map ∘ cpp_sort` follows rather than needing its own separate assertion.* This is the formal content of the "congruence" requirement in [[Equivalence and Witnesses]], and it is the property that makes a finite set of asserted equivalences useful for an infinite set of programs.

## Institutions: the right formalism for "many languages, one store"

There is an existing formalism for exactly this situation — Goguen and Burstall's [[Institution|institutions]], developed for algebraic specification across multiple logics. An institution provides signatures, models, sentences and satisfaction, plus the **satisfaction condition**: for a signature morphism $sigma : Sigma → Sigma'$,

$$ M' ⊨_(Sigma') sigma(phi) quad ⟺ quad sigma(M') ⊨_Sigma phi $$

"truth is invariant under change of notation." An *institution comorphism* is precisely a semantics-preserving translation from one language into another, which is what a Sophia frontend is supposed to be. The Hets toolset already implements this for dozens of logics. Whether this buys the project anything practical is uncertain, but it means the design space has been mapped before, and it supplies the correct vocabulary for what a frontend must guarantee.

## What this means concretely for the schema

1. **One node kind for terms**, with type/proof roles expressed as edges, not as separate tables. ([[Graph Schema]])
2. **Layer is a property of a node, not a separate store.** `Term`, `Op`, `Instr` are different kinds because their vocabularies differ, but they live in the same graph and the same hash space.
3. **Frontends must be functorial and must be tested for it.** A property test — "elaborate(f ∘ g) = elaborate(f) ∘ elaborate(g) for random f, g" — is a cheap, strong check on a frontend, and is worth building before anything else.
4. **Naturality is a checkable obligation on asserted equivalence sets**, not a philosophical aside. Given a set of `EQUIV` edges and a set of composition operators, the naturality squares are enumerable and testable.

## Related

- [[Curry-Howard-Lambeck Correspondence]]
- [[Functor]]
- [[Natural Transformation]]
- [[Initial Algebra]]
- [[Institution]]
- [[Core Calculus]]
- [[Equivalence and Witnesses]]
