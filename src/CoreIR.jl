"""
    Sophia.CoreIR

Julia-side mirror of the Sophia Core term language.

Design notes: `CoreIR.md`, `markdown/Design/Core Calculus.md`.
COMMENTS ONLY — nothing here is implemented.

# Why mirror a type that already exists in Rust

Two reasons, and the second is the real one.

1. Ergonomics. Building and inspecting core terms in the REPL is how the
   frontend gets debugged.
2. **Differential testing.** A second, independent implementation of the term
   representation and its canonical serialisation is the cheapest available
   check on the first. The Rust side is in the trusted computing base
   (`markdown/Design/Trusted Computing Base.md`); an independent Julia
   implementation that agrees on every digest over a large corpus is real
   evidence that the specification, not just the code, is what is being
   implemented.

The cost is a duplicated definition that can drift. Mitigation: generate both
from one schema file, or at minimum run the conformance suite in `Hashing.jl`
on every change.

# Intended representation

    abstract type Term end

    struct Var   <: Term; idx::Int end            # de Bruijn index
    struct Ref   <: Term; hash::Hash end          # free var: another definition
    struct Univ  <: Term; level::Int end
    struct Pi    <: Term; dom::Term; cod::Term; quantity::Quantity end
    struct Lam   <: Term; body::Term end
    struct App   <: Term; fun::Term; arg::Term end
    struct Let   <: Term; val::Term; body::Term end
    struct Fix   <: Term; body::Term end          # Cmp fragment only
    struct Prim  <: Term; op::PrimOp end

Immutable structs throughout: terms are values, never mutated. That matches the
append-only store and makes interning safe.

# Primitives keep their attributes

    @enum Overflow Wrap Trap Poison

    struct IntArith <: PrimOp
        op::Symbol            # :add, :sub, :mul, :sdiv, :srem, :shl, ...
        width::Int
        signed::Bool
        overflow::Overflow
    end

    struct FloatArith <: PrimOp
        op::Symbol
        fmt::Symbol           # :f32, :f64
        rounding::Symbol      # :rne, :rtz, ...
        contract::Bool        # may fuse into FMA?
        reassoc::Bool         # may reassociate?
    end

Julia's `+` on `Int64` is `IntArith(:add, 64, true, Wrap)`. C++'s signed `+` is
`IntArith(:add, 64, true, Poison)`. THEY ARE DIFFERENT NODES and must hash
differently — see `markdown/Design/Cross-Language Semantic Hazards.md`. The
frontend must never emit the wrong one out of convenience.

# Fragments

    @enum Fragment Prf Cmp

`Fix` is only legal in `Cmp`. Julia code is always `Cmp`. Specifications and
witnesses written with `@spec` live in `Prf`. The constructor should refuse to
build a `Prf` term containing `Fix` rather than leaving it to a later check.

# Pretty-printing

`show(io, ::MIME"text/plain", ::Term)` should print named, readable syntax by
resolving `Ref` hashes through the name table — de Bruijn indices are correct
and unreadable. A `--raw` mode for when the names are the thing under
suspicion.
"""
module CoreIR
end
