#overview

> The decisions recorded for Sophia, numbered `S-ADR-nnnn`. The `S-` prefix keeps them apart from Moonkale's ADRs, which this vault cites as "Moonkale ADR-00nn". A decision is `proposed` until Daniel accepts it. An accepted decision is not reopened without a new ADR that supersedes it. Use [[ADR Template]] for a new one.

| # | Decision | Status |
|---|---|---|
| [[S-ADR-0001 Sophia's first job is Moonkale's code store]] | Sophia's first goal is to be the store-backed code source Moonkale designs against | proposed |
| [[S-ADR-0002 Hash the core term, not the text]] | Identity is the hash of the canonical core term. Declared types count; docs, typings and proofs do not | proposed |
| [[S-ADR-0003 Identity is content, names and lineage]] | Three handles: content id, name, lineage | proposed |
| [[S-ADR-0004 Rust for the store, Julia for experiments]] | Rust only; Julia maybe much later for ML on the code graph | accepted |
| [[S-ADR-0005 SQLite first behind a store trait]] | SQLite as the first backend; others can be added behind the trait | proposed |
| [[S-ADR-0006 Edits through projections and the existing operations]] | Text is printed, edited and parsed back; Moonkale's existing `Op` set covers v0 | proposed |
