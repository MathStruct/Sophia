"""
    Sophia.Hashing

Julia-side hashing, and the conformance suite that keeps it byte-identical to
the Rust implementation.

Design notes: `Hashing.md`, `markdown/Design/Hashing and Identity.md`.
COMMENTS ONLY — nothing here is implemented.

# The contract

    hash_term(t::CoreIR.Term) :: Hash        # must equal sophia-hash's h_def
    hash_type(t::CoreIR.Term) :: Hash        #                        h_type
    hash_erased(t::CoreIR.Term) :: Hash      #                        h_run

"Must equal" is the entire point of this file. If the two implementations ever
disagree on a single term, one of them is wrong about the specification, and
the specification is what the whole store depends on.

`SHA` and `UUIDs` are already dependencies in `Project.toml`; BLAKE3 will need
a binding or a JLL. `XXhash` is for the in-memory intern table only and must
never reach a persisted digest (`markdown/Wiki/Hashing/Hash Consing.md`).

# The conformance suite — the reason this module exists

    "hashes agree with Rust" begin
        for t in corpus                       # generated + harvested terms
            @test hash_term(t) == ccall_rust_hash_term(t)
        end
    end

The corpus should contain, at minimum:

  * every `Term` constructor, including the degenerate cases
  * deeply nested terms (to catch recursion-depth and stack issues)
  * alpha-variants of the same term (must collide)
  * terms differing ONLY in a primitive attribute (must NOT collide) —
    especially `Wrap` vs `Poison` overflow and `contract` true/false
  * mutually recursive groups of 2, 3 and 5 definitions, to exercise the SCC
    construction (`markdown/Wiki/Hashing/Cycle Hashing.md`)
  * a symmetric SCC, where colour refinement cannot separate the members and
    the lexicographic tie-break has to fire
  * terms with 0.0 and -0.0, and with several NaN payloads (these must NOT
    collide; float attributes hash as bit patterns)
  * unicode identifiers and very long literals

# Property tests

    @test hash_term(t) == hash_term(deepcopy(t))                # determinism
    @test hash_term(t) == hash_term(rename_bound_vars(t))       # alpha-invariance
    @test hash_term(t) != hash_term(perturb_attribute(t))       # sensitivity
    @test canonicalise(canonicalise(t)) == canonicalise(t)      # idempotence

The third is the one people forget to write, and it is the one that catches an
over-eager canonicaliser — which is the failure mode with no other detector
(`markdown/Design/Trusted Computing Base.md`).

# Cross-machine determinism

Run the same corpus in CI on x86-64 and aarch64, on Linux and macOS, and
compare digests. Endianness, float printing, and hash-table iteration order
are the three classic sources of divergence, and all three are avoidable by
the encoding rules in `crates/sophia-hash/src/sophia_hash.rs`.

# Display

    show(io, h::Hash)  ->  "sophia:b3:qz3k…"     (base32, first 160 bits)

Truncation for reading only. A 128-bit UUID may be derived via `UUIDs` for
systems that demand one; it is never a key
(`markdown/Wiki/Hashing/UUID.md`).
"""
module Hashing
end
