//! `sophia` — the command-line driver.
//!
//! Design notes: `main.md`, `markdown/Design/Repository Layout.md`.
//! COMMENTS ONLY — nothing here is implemented. (This file previously held the
//! cargo-generated hello world; it was moved here from src/main.rs when the
//! workspace layout was introduced.)
//!
//! The CLI is how the graph is inspected before there is any IDE integration,
//! so its job is to make the store's contents legible to a human. Every
//! subcommand below corresponds to a query in
//! markdown/Design/Query Cookbook.md.
//!
//! ## Intended subcommands
//!
//! ```text
//! sophia ingest <lang> <path>        parse + elaborate + store
//! sophia show <hash|name>            pretty-print a definition and its type
//! sophia deps <hash> [--rev] [--depth N]
//!                                    dependencies / dependents
//! sophia context <hash>              THE query from Start Here: the definition
//!                                    plus its type, tests, docs, callers,
//!                                    callees, equivalents and source span
//! sophia equiv <a> <b> [--level L] [--modulo M...]
//!                                    assert, or search for, an equivalence
//! sophia check <hash>                re-run every witness touching this node
//! sophia build <hash> --target <t>   saturate, extract, lower, emit
//! sophia trust <artifact>            weakest link in the derivation, with the
//!                                    union of all moduli that were relied on
//! sophia test <hash> [--affected-by <h>]
//!                                    exact test selection, not heuristic
//! sophia gc [--dry-run]              reachability from Name roots
//! sophia migrate --to-version <v>    re-elaborate under a new hash schema
//! sophia verify                      rehash everything; the store is untrusted
//! ```
//!
//! ## Two of these are the actual demos
//!
//! `sophia context` is the capability markdown/Start Here.md predicts ("one
//! could query the exact context of a piece of code"). Today, assembling that
//! answer needs an IDE, a test runner, a coverage tool and a doc generator —
//! and still misses the alternative implementations, because nothing records
//! them. It is the most convincing thing to show someone early, and it does
//! not depend on any of the risky machinery.
//!
//! `sophia trust` is the honesty feature: it prints the weakest equivalence
//! level anything in the derivation relied on and the union of all moduli. A
//! system that cannot answer this should not be trusted to substitute code.
//! See markdown/Design/Trusted Computing Base.md.
//!
//! ## Output discipline
//!
//! Human-readable by default, `--json` for everything. Hashes print as
//! `sophia:b3:<base32, first 160 bits>` with a `--full` flag; truncation is
//! for reading, never for keys (markdown/Wiki/Hashing/UUID.md).
//!
//! ## What NOT to build here
//!
//! - No editing. Source stays in files and Git for the whole early roadmap
//!   (markdown/Design/Naming and Change Propagation.md); a Unison-style
//!   codebase manager fights every existing editor and reviewer.
//! - No daemon, no server, no LSP. Later, if ever.
//! - No "just make it work" flags that silently accept Asserted-level
//!   equivalences. If a build needs one, it must say so in its output and in
//!   the artifact's provenance.

fn main() {}
