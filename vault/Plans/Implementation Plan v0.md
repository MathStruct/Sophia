#implementation #design

> The order in which the base Sophia ([[SPEC-001 Base Sophia v0]]) would be built, with each phase's acceptance test. **Not started. It waits for Daniel's review of SPEC-001** (and for S-ADR-0004, the implementation language). Phases are small on purpose. Each one leaves something that runs.
>
> Sources: original to this vault. code: `crates/` (the existing comment-only crates are reused where their names fit).

## Crates

| Crate | Job | Existing? |
|---|---|---|
| `sophia-core` | the `Term` and `Definition` types of Core v0 (SPEC-001 §3) | yes, comment-only; rewritten |
| `sophia-hash` | canonical tag-length-value encoding, BLAKE3, SCC ordering (SPEC-001 §4) | yes, comment-only; rewritten |
| `sophia-store` | `Store` trait and its SQLite backend, names, log, lineage, the SQL views (SPEC-001 §5, §7, §11, §13) | yes, comment-only; rewritten |
| `sophia-syntax` | the canonical projection's printer and parser, source maps, diagnostics (SPEC-001 §6) | new |
| `sophia-cli` | the `sophia` command (SPEC-001 §14) | yes, comment-only; rewritten |
| `sophia-equiv`, `sophia-emit`, `sophia-frontend-metamath` | later or dropped | left untouched until decided |

On the Moonkale side, the `sources/sophia` adapter is Moonkale's work package (WP-68), built against `sophia-store` and `sophia-syntax` through a git or path dependency.

## Phases

| Phase | What | Acceptance |
|---|---|---|
| 1 | `sophia-core` + `sophia-hash`: terms, canonical encoding, digests, SCCs | property tests: α-invariance; determinism; a dependency change changes the dependant; the even/odd group shares a component; holes and unresolved names hash and differ |
| 2 | `sophia-store` on SQLite: put/get definitions, names, log, lineage, `migrated_to` | the fixture tests of SPEC-001 §14.1 that need no syntax; re-putting writes nothing |
| 3 | `sophia-syntax`: printer and parser for the canonical projection | round trip over the sample store; diagnostics with ranges; unresolved names come back as warnings |
| 4 | `sophia-cli` and the sample store (about 20 definitions) | SPEC-001 §14.2 and §14.3 |
| 5 | Moonkale adapter (Moonkale's side) | SPEC-001 §14.4 = WP-68's acceptance |
| 6 (v0.x) | a first checker for the non-dependent fragment, producing `sophia:Typing` with provenance | stale typings show as stale in Moonkale after an edit |
| 7 (v0.x) | `datalog` dialect, semi-naive, over the store's relations | "transitive dependants of `add`" in Moonkale's table and graph views |

## Later, not planned

Dependent checking; executing terms (an interpreter first); the relations/rules evaluator (shared with phase 7); proofs with checkers (with Moonkale WP-19); e-graphs; lowering to MLIR/LLVM; branches; update propagation (SP-002); projections from Moonkale's `[render]` settings (WP-30); other languages in the store.

## Risks to watch

- **The parser and printer round trip** is load-bearing for editing (S-ADR-0006). If it is hard to keep, editing moves to structural operations sooner (SP-004).
- **SQLite at 10⁶ definitions**: to be measured in phase 2 with a generated store, not assumed.
- **Scope creep in Core v0**: effects, arrays and dispatch are tempting, but they are not needed for WP-68.
