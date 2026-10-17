#design

> **Status: accepted (2026-10-17, Daniel): Rust only.** Proposed by Claude the same day; Daniel chose option 3. Related: [[Requirements]] SR-16, SR-30; [[Reading of the Earlier Vault]] item 1.

## Context

Daniel's Prompt3: "Julia for rapid development (also because I have no idea what we may do with such code graphs, and having all the numerical/scientific computing libraries available seems beneficial). Run Rust in production." The earlier [[Repository Layout]] read this as a split by concern: a Rust kernel plus a Julia frontend over a C ABI. Moonkale wants a Rust crate (SR-30).

## Options

1. **Julia first, then port to Rust.** Fastest to explore, but Moonkale cannot link it until the port.
2. **Rust for the store, hashing, syntax and CLI; Julia for experiments.** Julia reads and writes the same SQLite file (`SQLite.jl`), or calls the Rust library later. What Moonkale links is the production part.
3. **Rust only.**

## Proposal

Option 2. The store format (SQLite tables plus the canonical encoding) is the contract, so either language can produce definitions. A Julia prototype of a checker or a query can run against a real store before anyone ports it.

## Decision (Daniel, 2026-10-17)

**Option 3: implement in Rust only.** In Daniel's words, roughly:
- The idea behind Julia was that, once the core stands, one could run machine learning on the code graph: graph kernels, GNNs or other analytics, because Julia's ecosystem is good for that and Rust's is not.
- Some experimental work might be done in Julia, if there is a good reason, "maybe way later, but now not". Perhaps Sophia can introspect itself by then.
- The other old idea, writing a Julia skeleton for an agent to port to Rust, is "now entirely unnecessary".

## Consequences

- Every v0 crate is Rust. The Julia package skeleton (`Project.toml`, `src/*.jl`, comment-only) is not part of v0. Whether to delete it or keep it is still open.
- SQLite as the store format (S-ADR-0005) keeps the later Julia path open without any work now. A Julia analytics tool can read the store, or a later export, directly.
- [[Reading of the Earlier Vault]] item 1 is settled: the earlier split by concern (Rust kernel plus Julia frontend) is dropped.
