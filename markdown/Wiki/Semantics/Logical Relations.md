# Logical Relations

A proof technique for establishing properties of typed programs that are not provable by straightforward induction on terms — most importantly strong normalisation and [[Contextual Equivalence|contextual equivalence]].

The idea is to define a relation **by recursion on types** rather than on terms. For a binary relation on closed terms:

$$ (t, u) ∈ cal(R)_"Int" quad ⟺ quad t ⇓ n "and" u ⇓ n "for the same" n $$
$$ (t, u) ∈ cal(R)_(A → B) quad ⟺ quad ∀ (a, b) ∈ cal(R)_A. space (t space a, u space b) ∈ cal(R)_B $$

The function case is the crux: two functions are related when they map related arguments to related results. This makes the relation a **congruence by construction**, which is what sidesteps the impossible quantification over all contexts in the definition of contextual equivalence. The *fundamental theorem* — every well-typed term is related to itself — then does the real work.

Extensions, each needed for a realistic language: **step-indexing** (Appel–McAllester) to handle recursive types without a circular definition; **Kripke logical relations** indexed by a world, for state and for local invariants; **biorthogonality** for control effects.

The reason this appears in a project about cross-language equivalence: a logical relation is exactly the mathematical object that a *language-pair relation* `R` must be ([[Equivalence and Witnesses]]). Relating `Int64` in Julia to `int64_t` in C++, `Array` to `vector`, and Julia functions to C++ functions, with the function case saying "related arguments go to related results" — that *is* a cross-language logical relation, and the fact that it is a well-studied object is the strongest evidence that the goal is coherent rather than hand-waving.

## Related

- [[Contextual Equivalence]]
- [[Bisimulation]]
- [[Equivalence and Witnesses]]
- [[Multi-AST Layering]]
