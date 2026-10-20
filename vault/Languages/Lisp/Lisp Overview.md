#overview

> Lisp in Sophia: planned, not started. [[S-ADR-0008 Sophialang before Metamath or Lisp]] put sophialang first; Metamath came in as data (B-16), Lisp has not. 2026-10-20.
>
> Sources: [[S-ADR-0008 Sophialang before Metamath or Lisp]]; [[Operating Systems and Sophia]] (CLOSOS: first-class global environments); Flatt, *Binding as Sets of Scopes*, POPL 2016 (hygiene).

| Level | Status |
|---|---|
| L1–L6 | not started |

**Why Lisp is interesting for Sophia:**
- **Homoiconicity**: code is data, so a Lisp's canonical form is nearly its reader output, the easiest L1/L2 of any language.
- **Macros everywhere**: the same question as Julia's macros ([[Julia Macros and Generated Functions]]), in its purest form. Hygiene (sets of scopes) is the precise account of what a macro's names mean.
- **First-class environments** (CLOSOS, Common Lisp packages): the same idea as Sophia's per-environment worlds (SPEC-006 §2).

**Which Lisp** is undecided: Scheme (small, R7RS, hygienic macros), Common Lisp (packages, CLOS generic functions with multiple dispatch, the closest relative of Julia's dispatch), or Clojure. Daniel's call when it becomes relevant.
