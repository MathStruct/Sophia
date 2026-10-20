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
| [[S-ADR-0007 The store is language-agnostic]] | Sophia (the store) and sophialang (its language) are separate; the store serves every language | accepted |
| [[S-ADR-0008 Sophialang before Metamath or Lisp]] | The first language is sophialang's core | accepted |
| [[S-ADR-0009 Relations are tracked, not content-addressed]] | Relations get a fixed identity; rules attach like methods | accepted |
| [[S-ADR-0010 Primary backend and an analytics replica]] | SQLite the default for now; LMDB experimental; no permanent choice before B-15, B-16, B-17 | accepted in amended form (Daniel's forwarded direction) |
| [[S-ADR-0011 Sophia has its own query algebra]] | A storage-independent query algebra (stratified Datalog); SQL is a translation target | accepted |
| [[S-ADR-0012 The query IR is a separate IR on shared primitives]] | Queries are their own IR on the store's primitives now; a quoted core node later | accepted |
