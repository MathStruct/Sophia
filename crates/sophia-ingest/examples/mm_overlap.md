#implementation

> An experiment: how much do set.mm and iset.mm share? Source: `mm_overlap.rs`. Run: `cargo run --release -p sophia-ingest --example mm_overlap -- ~/.cache/sophia-corpora/set.mm`.

Result (2026-10-20):
- 13 138 labels occur in both libraries; **12 927 of them are the same statement**, and the other 211 are genuinely different statements (for example iset.mm's `ax-8`).
- 13 180 iset.mm statements occur somewhere in set.mm, under any label.
- 121 iset.mm proofs are identical to set.mm proofs all the way down. Proof identity covers the whole chain of proofs, and iset.mm's foundations differ.
