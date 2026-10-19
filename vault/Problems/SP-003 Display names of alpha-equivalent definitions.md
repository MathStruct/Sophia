#open-problem

> **Status: accepted wart (2026-10-17).** Related: [[SPEC-001 Sophia Store v0]] §4.6; [[Alpha Equivalence]].

**Problem.** `f(x) = x + 1` and `g(y) = y + 1` hash the same (locals are de Bruijn indices, and the own name is excluded). The store keeps one set of binder display names, so `g` prints as `g(x) = x + 1`.

**Why it is accepted.** Unison has the same behaviour, and it rarely matters. A fix would be display names keyed by (name, binder path) instead of (content, binder path), so that each *name* remembers its author's spelling. That is a small, later change, because display metadata is outside the hash.
