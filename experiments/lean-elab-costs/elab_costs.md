#measurement #lean

> B-035 part 2: a Lean driver that elaborates one Lean file command by command through Lean's own frontend (`Lean.Elab.Frontend.processCommand`), synchronously, and prints the time, heartbeats and constants added of each source command. It gives the cost of the expensive facts of the fact cache experiment ([[SPEC-001 Sophia Store v0]] §8a). Run by `crates/sophia-ingest/examples/lean_elab_costs.rs` ([[lean_elab_costs]]). Results: [[Fact Cache Experiment]] §Part 2.
>
> Sources: Lean v4.36.0-rc1, `src/lean/Lean/Elab/Frontend.lean`. code: `elab_costs.lean`

```sh
cd ~/Code/CategoryTheory/mathlib4
PATH=~/.elan/bin:$PATH lake env lean --run PATH/TO/elab_costs.lean Mathlib/X/Y.lean Mathlib.X.Y
```

**Output:** one TSV row per source command, `index line kind decl_kind constants_added errors ns heartbeats`, and first a `#header` row with the time spent on the imports. `kind` is the command's syntax kind; `decl_kind` is the keyword's kind under a `declaration` (theorem, definition, instance, ...), looking through `set_option ... in` and the other `... in` prefixes.

**What it does like `lean FILE`:** it parses the header, imports through `processHeader` with the module's name as `mainModule` and `leakEnv := true` (as the command line does; before this, early commands of a file paid a one-time cost of up to about 20 ms, most likely for this reason: the same run also added the waits below), and runs `processCommand` until the end of input. `Elab.async` is set to false, so a theorem's proof is elaborated inside its command and charged to it, as `lean -DElab.async=false FILE` does. Before a command's timer stops, the driver waits for the kernel environment (`env.checked`) and for the command's snapshot tasks, so work the command started but did not finish is charged to it.

**What it does differently.** It uses the older `Frontend.processCommand` loop, not the `Language.Lean` snapshot processor of the command line, and it does not run the module linters at the end of the file. Its trust level is 1024. Its times are about 7 % above `lean`'s (the median over the B-035 sample, [[Fact Cache Experiment]]); the cause is not known.

**Why not the trace profiler.** `lean -Dtrace.profiler=true` prints one `[Elab.command]` line per command, but a command that a macro expands into several (Mathlib's `theorem LiftRel.equiv` becomes `namespace LiftRel`, `theorem equiv`, `end LiftRel`) appears as each expansion **and** the source command, each at top level. Summing the lines counts the work twice: `Mathlib/Data/WSeq/Relation.lean` shows 145 lines for 68 source commands. With `trace.profiler.threshold=0` the profiler also inflated command times by about 14 % (`Mathlib/Data/Nat/Factorial/Basic.lean`, commands of 10 ms or more: 0.671 s against 0.587 s at threshold 10 ms) and made the run 18× slower.

**Checks:**
- **errors:** the error messages of each command. `elabCommandTopLevel` resets the message log per command, so the log after a command is that command's alone. The fixture `fixtures/TwoErrors.lean` has two failing commands in a row; each must show its errors (2 each with v4.36.0-rc1): `lean +leanprover/lean4:v4.36.0-rc1 --run elab_costs.lean fixtures/TwoErrors.lean TwoErrors`. The runner stops on any error.
- **wall time:** the runner also times `lean -DElab.async=false FILE`, for comparison with the driver's import and command times.

**What it does not measure:**
- The kernel's check is included where Lean runs it inside the command (`addDecl`), so it is part of the cost; linters run after each command and are included too.
- The asynchronous mode Lake uses by default spreads proofs over threads. Total work is the same order; the per-command split is what is wanted here.
- The first command of a file pays some warm-up (the first `moduleDoc` of `WSeq/Relation.lean`: 1.2 ms).
- Heartbeats are Lean's count of small allocations (`IO.getNumHeartbeats`), a deterministic companion to the time.
