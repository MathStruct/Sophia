# State of the Art - Incremental Computation

The property [[Start Here]] wants from content addressing — "same hash ⇒ already compiled" — is an instance of incremental computation, and there is a mature body of work on doing it well. This is also the area with the clearest, nearest-term payoff, via [[Content-Addressed Precompilation]].

## Demand-driven / query-based compilers

The dominant modern architecture, and the one [[Compilation as Query]] is describing:

- **Salsa** (Rust) — the incremental framework underneath `rust-analyzer`. Computation is expressed as memoised *queries*; dependencies are recorded automatically by observing which queries each query calls; invalidation uses **red-green marking** with early cutoff (if a recomputed input produces the same value, dependents are not recomputed). Early cutoff is exactly what content-addressing gives for free.
- **rustc's query system** — the compiler itself, restructured around demand-driven queries with an on-disk incremental cache. A production-scale demonstration that a compiler can be a memo table.
- **Adapton** — self-adjusting computation with a general theory of change propagation; the academic counterpart.
- **Incremental λ-calculus** (Cai, Giarrusso et al.) — derivatives of programs with respect to input changes, the most principled treatment.
- **Skip** — a language designed from scratch around incremental computation.

## Build systems

- **"Build Systems à la Carte"** (Mokhov, Mitchell, Peyton Jones) — the definitive taxonomy, factoring build systems into a *scheduler* (topological / restarting / suspending) and a *rebuilder* (dirty bit / verifying traces / constructive traces / deep constructive traces). Reading Sophia through this lens: it is a **suspending scheduler with deep constructive traces**, i.e. the Nix/Bazel quadrant, applied at statement granularity.
- **Nix** — content-addressed derivations, a shared binary cache, and full reproducibility as the default. The design Sophia's cache should imitate.
- **Bazel** — remote caching and remote execution keyed on action hashes, at industrial scale. The Remote Execution API is a ready-made protocol for a shared compile cache.
- **ccache / sccache** — the pragmatics of a compiler cache, including the ugly parts: what to do about `__DATE__`, absolute paths, and headers.
- **Rattle**, **Shake**, **Buck2** — the modern experimental end; Buck2 in particular is worth studying for its handling of dynamic dependencies.

## Where Julia sits today

`pkgimages` ([[Pkgimage]]) are a verifying-trace rebuilder at package granularity. [[World Age|Backedges]] are a dirty-bit mechanism at method granularity, but they live only in memory and are discarded at process exit. The gap [[Content-Addressed Precompilation]] targets is exactly *persisting the traces that already exist ephemerally*, which the "Build Systems à la Carte" framing makes precise: Julia has constructive traces in memory and verifying traces on disk, and no deep constructive traces anywhere.

## The lesson for Sophia

1. **Early cutoff is the whole game.** A change that does not change a value must not propagate. Content hashing gives this by construction, which is why the architecture is right even though the storage layer is risky.
2. **Dependency tracking must be automatic**, not declared. A graph store where dependencies *are* the edges gets this for free — arguably the single strongest structural argument for the project.
3. **Shared caches need content addressing to be trustworthy**, which is exactly [[Trusted Computing Base]]'s point that the store need not be trusted.
4. **Granularity is the differentiator.** File-level is the status quo; statement-level is the bet.

## Related

- [[Content-Addressed Precompilation]]
- [[Compilation as Query]]
- [[Pkgimage]]
- [[State of the Art - Content-Addressable Code Systems]]
- [[State of the Art - Julia Compilation and Precompilation]]
