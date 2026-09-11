# Hashing and Identity

[[Start Here]] asks for "unique reproducible UUIDs calculated from the tree of dependencies". This note makes that precise, including the three parts that are not obvious: binders, cycles, and what is deliberately left *out* of the hash.

## The basic construction

For a node `n` with ordered children `c_1 … c_k`:

$ h(n) = H("ver" ‖ "tag"(n) ‖ "attrs"(n) ‖ h(c_1) ‖ … ‖ h(c_k)) $

where `H` is [[BLAKE3]], `‖` is length-prefixed concatenation (never naive concatenation — otherwise `("ab","c")` and `("a","bc")` collide), `tag` is the node kind, and `ver` is the *hash schema version* (see below). This is a [[Merkle DAG]] in exactly the sense Git and Nix use one, applied at statement granularity rather than file or package granularity.

Because children are folded in by hash rather than by name, the identity of a definition transitively encodes its entire dependency tree. Renaming anything changes nothing.

## Binders: hashing must be α-invariant

Names of bound variables are not semantically meaningful, so they must not reach `H`. Before hashing, terms are converted to a **locally nameless** representation: bound occurrences become [[De Bruijn Index|de Bruijn indices]], free occurrences remain named (by hash of their definition). This gives

$ t ≡_alpha u ⟹ h(t) = h(u) $

which is the minimum bar. Note the converse is *not* claimed and does not hold in general — see the normalisation step below for how much further we push it.

## Canonicalisation before hashing

Raw structural hashing is too brittle: `a + b` and `b + a`, or two `let`-blocks differing only in binding order, would get different identities despite being the same program. So hashing operates on a **canonical form**, not on the raw elaborated tree. The canonicaliser applies, in order:

1. α-conversion to locally nameless form
2. `let`-floating and dead-binding elimination
3. η-expansion to a fixed arity convention
4. weak-head normalisation, then full [[Normalization by Evaluation|normalisation]] of the type-level parts (definitional equality *must* be reflected in identity, or the conversion rule and the hash disagree)
5. deterministic ordering of genuinely unordered structures — record fields, module members, `Σ`-telescope-independent bindings — by the hash of their contents (a fixpoint, since the ordering key depends on hashes that depend on the ordering; resolved by iterating to stability)
6. attribute normalisation for primitives (explicit rounding modes, explicit overflow discipline)

Everything the canonicaliser does is a *decision*, and every decision is a commitment: changing it changes every hash downstream. See the versioning section.

**It does not normalise commutativity, associativity, or any algebraic law.** Those belong in the [[E-Graph|e-graph]] as equivalences, not in the hash. Trying to bake them into canonical form leads directly to needing a decision procedure for program equality, which does not exist.

## Cycles: mutual recursion breaks Merkle hashing

`h` as defined above is only well-founded on a DAG. Mutually recursive definitions form cycles, and Unison hit this problem too. The construction:

1. Build the dependency graph; find strongly connected components (Tarjan).
2. Contract each SCC to a single unit. The condensation is a DAG, so `h` is well-defined *on SCCs*.
3. Within an SCC, members must be given a canonical order that does not depend on their names. Do this by iterative refinement: start with a colouring by node tag and by the hashes of all *external* (non-SCC) dependencies, then repeatedly refine each member's colour by the multiset of colours of its SCC-neighbours until the partition stabilises — the standard [[Cycle Hashing|1-dimensional Weisfeiler–Leman]] / Paige–Tarjan refinement.
4. If refinement leaves a non-singleton class, the members are genuinely symmetric; break the tie by the lexicographically smallest ordering of their serialisations, and record the choice.
5. $ h("scc") = H("ver" ‖ "SCC" ‖ h_1^"ref" ‖ … ‖ h_m^"ref") $, then each member gets $ h_i = H(h("scc") ‖ i) $.

Step 4 is where the theoretical ugliness lives — canonical labelling of a graph is not known to be polynomial in general. In practice SCCs in real code are tiny (2–5 mutually recursive functions) and refinement separates them immediately, so a brute-force tie-break is affordable. This is a place where an adversarial input could be expensive; SCC size should be capped with a hard error.

## Three hashes per definition, not one

Collapsing everything into one identifier throws away distinctions the system needs:

| Hash | Computed over | Used for |
| --- | --- | --- |
| $h_"def"$ | the fully elaborated core term, types and all | node identity; the "UUID" of [[Start Here]] |
| $h_"type"$ | just its type, canonicalised | interface compatibility, dispatch, ABI |
| $h_"run"$ | the type-erased executable term | codegen cache key — see [[Content-Addressed Precompilation]] |

$h_"run"$ is the interesting one: two definitions that differ only in erased type information (a common outcome of dependent elaboration) share a $h_"run"$ and therefore share compiled machine code. That is exactly the cache-hit rate improvement the Julia precompilation motivation was after.

## What is *not* in the hash

Deliberately excluded, and attached as metadata edges instead ([[Graph Schema]]):

- names, namespaces, module paths
- doc comments, markdown, annotations ([[Tests and Documentation as Nodes]])
- source file, byte span, line numbers
- author, timestamp, provenance
- which frontend produced it
- test results and benchmark results
- optimisation level, target triple

The rule is: *if it does not change what the program means, it does not change the identity.* The consequence is that the same definition extracted from Julia and from C++ — if they really do elaborate to the same core term — gets the **same hash automatically**, and no equivalence edge is needed at all. Equivalence edges are only needed when the core terms genuinely differ.

## Hash function and identifier format

- **[[BLAKE3]]**, 256-bit output. Chosen for speed on the large volume of small inputs this workload generates, for its built-in tree structure, for keyed-hash domain separation, and for XOF output if shorter identifiers are wanted.
- **Domain separation**: each node tag hashes under its own derived key, `H_k` with `k = derive_key("sophia.node." + tag)`. This makes cross-tag collisions structurally impossible rather than merely improbable.
- **Non-cryptographic hashing** (`xxhash`, already a dependency in `Project.toml`) is used only for in-memory [[Hash Consing|interning]] and is never persisted or trusted.
- **Display form**: `sophia:b3:<base32-of-first-160-bits>`, with the full 256 bits stored. A 128-bit [[UUID]] can be derived by truncation for systems that demand UUID-shaped keys (`UUIDs` is likewise already a dependency), accepting the weaker collision bound.

Collision probability for `n` distinct objects at 256 bits is about $n^2 slash 2^257$; for $n = 2^40$ (a trillion nodes, far beyond anything realistic) this is $≈ 2^(-177)$. Truncating to 128 bits for UUID display gives $≈ 2^(-49)$ at the same `n`, which is why truncation is for display only.

## Versioning the hash schema

Any change to the canonicaliser, the tag set, the attribute encoding or `H` changes every hash in the database. This is not avoidable, so it is made explicit:

- `ver` is the first field hashed, so different schema versions inhabit disjoint hash spaces by construction.
- The store holds multiple versions side by side; a `MIGRATES_TO` edge relates the old and new identity of the same definition.
- Migration is a re-elaboration, not a rewrite: re-run the canonicaliser at the new version over the old core terms.
- This is what [[Start Here]] means by "rewrite the core of the graph compiler and migrate the database to his own preferred schema" — and it is genuinely cheap *because* the source of truth is core terms, not text.

## Related

- [[Merkle DAG]]
- [[De Bruijn Index]]
- [[Alpha Equivalence]]
- [[Hash Consing]]
- [[BLAKE3]]
- [[Cycle Hashing]]
- [[Content-Addressed Code]]
- [[Graph Schema]]
- [[canonical]] and [[merkle]] — the Rust modules that would implement this
