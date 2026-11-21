# Review brief: B-035 part 2 measurement programs (defects only)

Repository: /home/daniel/Code/CategoryTheory/Sophia (read-only for you; do not edit, commit or build in it except as below).

## Requirement
Measure what Lean 4's elaborator spends per source command, and per declaration, on a deterministic sample of Mathlib files
(Mathlib at ~/Code/CategoryTheory/mathlib4, Lean v4.36.0-rc1 under ~/.elan; Lean sources at
~/.elan/toolchains/leanprover--lean4---v4.36.0-rc1/src/lean). The numbers decide the cost threshold of a fact cache: the share of
declarations, and of elaboration time, at or above 0.01/0.1/1/10/100/1000 ms. Each command's time must include its proof
(synchronous elaboration), must not count macro expansions twice, and must match what `lean -DElab.async=false FILE` spends.

## Files to review
- experiments/lean-elab-costs/elab_costs.lean (Lean driver: elaborates one file command by command, prints TSV rows) and its note elab_costs.md
- crates/sophia-ingest/examples/lean_elab_costs.rs (samples files, runs `lean` and the driver, aggregates) and lean_elab_costs.md

## What to look for
Defects only: wrong results (timing that misses or double-counts work, wrong declaration classification, wrong quantiles or shares,
wrong sample), a mismatch between what the driver does and what `lean FILE` does (options, header processing, mainModule, async
work that escapes the timed region, e.g. tasks still running after processCommand returns), panics, claims in the two notes the code
does not support, missing tests. Not style.

You may run: `cd ~/Code/CategoryTheory/mathlib4 && PATH=~/.elan/bin:$PATH lake env lean --run /home/daniel/Code/CategoryTheory/Sophia/experiments/lean-elab-costs/elab_costs.lean Mathlib/Data/WSeq/Relation.lean Mathlib.Data.WSeq.Relation`
on a few files (each takes a few seconds, ~3 GB RAM; at most one at a time). Results of the 60-file run are in
~/.cache/sophia-corpora/b035-lean/run1/ (commands.tsv, files.tsv, summary.tsv, stdout.txt, stderr.txt). Do not run cargo.

## Answer
A numbered list of findings, most severe first; for each: file:line, the defect, the evidence (a command and its output where you
ran one), and a suggested fix. Mark what you verified apart from what you suspect. If you find nothing, say so.
