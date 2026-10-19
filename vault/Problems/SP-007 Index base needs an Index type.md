#open-problem

> **Status: open (2026-10-20).** Related: [[SPEC-005 Rendering Profiles]] §4.

**Problem.** Daniel wants 0- or 1-based indexing to be a display setting (SR-28). A printer can only shift an index correctly when it *knows* the integer is a position. `a[i+1]` must print as `a[i+1]` in both bases if `i` is a position, but `n+1` (a count) must never be shifted.

**Proposal.** A distinct `Index` type for positions:
- literal positions are stored as offsets;
- `Index ± Int64` is base-independent;
- conversions to and from `Int64` are explicit and print with their base.

**Costs.**
- Code that mixes counts and positions needs conversions. This is familiar from Rust's `usize` and Julia's `CartesianIndex`, but it is friction.
- Imported Julia code (1-based) and Rust code (0-based) use plain integers. Their positions can be recovered only by inference, and are otherwise displayed with a "base unknown" marker.

**Why it might fail:** if the friction makes people avoid `Index`, the display setting only works for a fraction of the code. Then the setting should apply only to code written with `Index`, and say so.
