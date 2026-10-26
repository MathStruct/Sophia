#open-problem

> **Status: open (2026-10-25).** Area: coalgebra, graph canonisation, hashing · Prerequisites: strongly connected components, bisimulation, DFA minimisation · Related: [[Theory of the v0 Store]] §1, [[component]], [[Cycle Hashing]], [[Hashing and Identity]].
>
> Sources: Hopcroft, *An n log n algorithm for minimizing states in a finite automaton*, 1971; Paige & Tarjan, *Three partition refinement algorithms*, SIAM J. Comput. 16 (1987); Moonkale Code Graph Schema, "WP-66 normalizer v1 → Recursive components" (the ordering rule Sophia uses); the question is original to this vault. code: `crates/sophia-hash/src/component.rs`.

## Question

Mutually recursive definitions are hashed as one group: the members are put in a canonical order,
and each gets an id from the group's digest and its position. Today the order quotients the group
by its **automorphisms**: members that a symmetry of the group swaps get one id. Members that
behave identically without being related by a symmetry keep different ids. **Which equivalence
should ids respect: automorphism, bisimilarity within a group, or equality of infinite unfoldings
across the whole store? For each, is there a canonical form that is computable in near-linear time
and that keeps the store a Merkle DAG, so that adding a definition never changes an existing id?**

## Setting

A **group** is a strongly connected set of members $0..n-1$. Member $m$ has a body $b_m$: its
canonical bytes with every reference to a member left as a hole, and its references to members
$r_m = (r_{m,1}, …, r_{m,k_m})$ in stream order. References outside the group are already digests
and belong to $b_m$. So a group is a deterministic transition system: state $m$ outputs $b_m$, and
transition $j$ goes to $r_{m,j}$.

- **Automorphism:** a permutation $\sigma$ with $b_{\sigma m} = b_m$ and $r_{\sigma m, j} = \sigma(r_{m,j})$.
- **Bisimilarity:** the largest relation $\sim$ with $m \sim m' \Rightarrow b_m = b_{m'}$ and
  $r_{m,j} \sim r_{m',j}$ for all $j$. For deterministic systems this is equality of the infinite
  unfoldings (regular trees).
- **The current rule** (`canonical_order`): for each member as root, number the members by
  breadth-first first visit along $r$, encode, concatenate; the smallest serialisation wins. Roots
  that tie are related by an automorphism, and their members are merged into one class.

## What is known

- **Measured (tests):** automorphic members are merged: `f() = g(); g() = f()` gets the bytes of
  `h() = h()`. Bisimilar members that no automorphism relates are not:
  `a = s(b, c); b = s(a, a); c = s(a, a)` unfolds like `h = s(h, h)` but keeps 3 classes against 1.
  `cargo test -p sophia-hash --lib component` (tests `automorphic_members_form_one_class`,
  `bisimilar_members_that_are_not_automorphic_stay_apart`, and `partial_symmetry_keeps_the_rest_distinct`,
  whose `a` and `b` are bisimilar too).
- **Proved, by construction:** the current rule costs $O(n \cdot |group|)$ bytes (one serialisation per
  root), bounded by a `Budget`.
- **By construction, not tested:** across groups nothing is merged. `x = s(y)` with `y = s(y)` unfolds
  like `y`, but `x` is a separate record whose bytes reference `y`'s digest.
- **Stale notes:** [[Hashing and Identity]] and [[Cycle Hashing]] describe colour refinement and say
  that it computes bisimilarity. The code does not refine colours. 1-dimensional Weisfeiler–Leman with
  neighbour *multisets* would not compute bisimilarity for ordered references anyway.
- **Expected, to be proved:** minimise the group first (Hopcroft's partition refinement over the
  outputs $b_m$ and the numbered transitions, $O(n \log n)$), then apply the current rule to the
  quotient. In a minimal deterministic system, two roots with equal serialisations are bisimilar and
  therefore equal, so no ties remain. The result would be a complete invariant for bisimilarity
  within a group.

## What would count as an answer

1. A proof that minimise-then-order gives equal serialisations exactly for bisimilar groups, with its
   cost.
2. For the store-wide level: either a construction whose ids are invariant under equal unfoldings
   while every existing id stays unchanged when a definition is added, or a proof that none exists.
   For example, `x`'s id would have to equal `y`'s, so it could no longer be a Merkle hash of `x`'s own
   bytes.
3. A measurement: how many groups in real code (LinearAlgebra, Moonkale, Lenticulum.jl) have bisimilar
   members that are not automorphic. The expectation is very few, which would make this a question of
   principle, not of storage.

## Why it matters here

Identity is the store's foundation (SR-01). The rule is shared with Moonkale (SR-116), so changing it
is a migration in both tools (SPEC-006 §6) and a decision for Daniel and Moonkale. [[Theory of the v0
Store]] argues that merging automorphic members is correct because they are interchangeable. Bisimilar
members are interchangeable too, so the same argument asks for more than the code does.

## Leads

- Moore-machine minimisation, then breadth-first numbering from the least root.
- Unique representations of regular trees and hash-consing of cyclic terms. Mauborgne's incremental
  representation of regular trees (Nordic J. Computing, 2000) is a lead to check, not yet read.
- Unison orders groups by their members' hashes with the cycle removed ([[Adopting from Unison]]).
  That is a different trade-off, and also not bisimulation-invariant.
