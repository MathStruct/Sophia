Review written to [scratch/ANSWER.md](/home/daniel/Code/CategoryTheory/Sophia-codex/scratch/ANSWER.md), with verified findings first:

- `facts` leaves nested lists uncapped and reports `truncated: false`.
- Startup can create a store from an empty SQLite file—pre-existing behavior that violates the requirement.

Reproducers and outputs are included. All eight feature combinations built; default and all-feature tests passed. `target/` is 26 GB. No source changes or commits.