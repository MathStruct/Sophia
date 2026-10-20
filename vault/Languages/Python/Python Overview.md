#overview

> Python in Sophia: identified through Moonkale's WP-66 normal form, resolved only within a file. No corpus planned; Python is supported because Moonkale supports it. 2026-10-20.
>
> Sources: Moonkale `moonkale-content-id` (Python normal form); `crates/sophia-ingest/src/code.rs`; `tests/code.rs`; the Python Language Reference, *Execution model* (scoping, `global`, `nonlocal`).

| Level | Status | Evidence |
|---|---|---|
| L1 Parsed | yes: tree-sitter-python via arborium | golden fixtures |
| L2 Normalised | yes: WP-66, identical ids to Moonkale; methods inside classes are definitions | `tests/code.rs` |
| L3 Resolved | **same-file names only**, by spelling | — |
| L4–L6 | no | — |

**What makes Python hard for Sophia:** everything is mutable at run time (module globals, class attributes, monkey-patching), imports execute code, and dispatch is through attribute lookup and the MRO. Static resolution can only ever approximate; a conformance oracle would be a running interpreter (as for Julia), or a type checker (pyright) for the statically typed subset.
