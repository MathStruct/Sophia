#implementation #measurement #lean

> B-035 part 2, the expensive-fact workload of the fact cache experiment ([[SPEC-001 Sophia Store v0]] §8a, SR-152). It draws a deterministic sample of Mathlib files, elaborates each with the Lean driver [[elab_costs]], and summarises what each command and each declaration costs: quantiles, and the share of commands and of time above thresholds from 0.01 ms to 1 s. It changes nothing in Mathlib. Source: `lean_elab_costs.rs`. Results: [[Fact Cache Experiment]] §Part 2.

```sh
cargo build --release -p sophia-ingest --example lean_elab_costs
PATH=~/.elan/bin:$PATH ./target/release/examples/lean_elab_costs OUTDIR ~/Code/CategoryTheory/mathlib4 \
    experiments/lean-elab-costs/elab_costs.lean [--sample 60] [--seed 35]
```

- **The sample:** every `.lean` file under `Mathlib/`, sorted; the `N` files with the smallest FNV-1a hash of (seed, path). The same seed gives the same files for the same Mathlib revision.
- **Per file, three runs**, all with `Elab.async = false` and through `lake env` in the Mathlib root: `lean FILE` to warm the page cache, the driver for the per-command rows, and `lean FILE` again for the wall time (a cross-check). Any error, from `lean`'s exit status or in a driver row, stops the run.
- **A declaration** is a command whose syntax is Lean's `declaration` (theorem, def, instance, abbrev, structure, class, inductive, example, opaque, axiom), through `... in` prefixes, or any command that added a constant (Mathlib's `irreducible_def`, `alias`, `initialize_simps_projections`, `mutual`, ...). Everything else (`namespace`, `variable`, `open`, docs, an `attribute` that adds no constant) is bookkeeping and is counted under "all commands". The unit is the source command: one that a macro turns into many declarations counts once.

**Output in OUTDIR:**
- `commands.tsv`: one row per command: file, index, line, kind, decl_kind, declaration, constants added, errors, ms, heartbeats;
- `files.tsv`: one row per file: lines, `lean` wall time, import time, commands, declarations, their summed times, errors;
- `summary.tsv`: per threshold, the count and time share of declarations and of all commands at or above it, the time also as a share of all command time;
- stdout: the same summary, quantiles, and the 15 costliest command kinds.

**Tests:** `cargo test --release -p sophia-ingest --example lean_elab_costs` checks the row parser (malformed rows and reported errors are refused), the declaration rule, the arguments (a missing value, `--sample 0`) and that the sample is deterministic.
