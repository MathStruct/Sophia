#implementation

> Frontends that bring real code into Sophia (B-16, [[Real Code Corpora]]). Metamath first. Source: `sophia_ingest.rs`.
>
> Sources: [[Real Code Corpora]]; [[SPEC-006 Worlds, Artifacts and Evidence]] §1. code: `sophia_ingest.rs`, `metamath.rs`

`store_metamath` stores a parsed database:
- **statements:** one record each, named `mm.<db>.<label>`;
- **proofs:** one record each, named `mmproof.<db>.<label>`;
- **links:** a `proof` annotation on each statement and its proof;
- **the library:** three nominal collections, `mmlib.<db>.proofs`, `.axioms` and `.definitions`, filled with `Store::add_members`.

Being an axiom, or the proof chosen for a statement, is a **library's** choice, not a property of the statement. One statement can be an axiom in one library and proved in another, or even in another part of the same library: set.mm's personal sections prove `ax-1` from other axiom systems.

`examples/mm_overlap.rs` compares set.mm and iset.mm (statements and proofs shared).
