#measurement #lean

> E-10 (B-029): re-checks Mathlib here with Lean's own kernel (`leanchecker`, shipped in the v4.36.0-rc1 toolchain), one process per module, and records exit code, time and peak memory per module. Turns "checked on Mathlib CI" into "checked on this machine" for Mathlib's own modules. Results: [[Lean Overview]] §E-10.
>
> Sources: lean4checker (now `leanchecker` in the Lean toolchain). code: `recheck.sh`

```sh
experiments/lean-recheck/recheck.sh $SOPHIA_CORPORA/e10/recheck-all 6                     # every Mathlib module
LEANCHECKER_FLAGS=--fresh experiments/lean-recheck/recheck.sh OUT 4 @modules.txt          # a list, replaying all imports
```

- Needs Mathlib with its build (`sophia-mathlib`); finds the toolchain from `mathlib4/lean-toolchain` under `~/.elan`. No `nix develop` needed.
- `OUT/results.tsv`: module, exit code, seconds, peak KB. `OUT/failures.txt`: the output of each failing module. `OUT/run.txt`: the count, flags, start and end.
- **Without `--fresh`** a module's own declarations are replayed through the kernel onto its imports, which are loaded without re-checking. Checking every module therefore covers all of Mathlib's declarations; Batteries, Aesop, Qq, Plausible and Lean's core are imported, not checked. **With `--fresh`** the module and everything it imports are replayed from nothing (dependencies included). `Mathlib.Logic.Hydra`: 3.3 s without, 76 s with.
- **Does it catch anything?** A test project (`$SOPHIA_CORPORA/e10/negtest`) adds `bad : False := True.intro` with `debug.skipKernelTC` and builds. `leanchecker Neg.Bad` exits 1 ("… but it is expected to have type False"), and `Neg.Good` exits 0.
- **Known crash:** `leanchecker` without `--fresh` segfaults (exit 139, no output) on 119 of Mathlib's 8 650 modules. The segfault is in `lean_name_eq` under `findOLean` while the imports' IR is read, before anything is replayed. On `Mathlib.Order.Circular` it is deterministic and does not depend on the stack size; `leanchecker-paranoid` crashes the same way, and `lean` itself imports the module. The 119 are re-checked with `--fresh`. The cause is not found; it is not related to the `module` header or `meta import`.
- Memory: up to 4.7 GB per process without `--fresh`, about 2 to 3 GB with it. Run it in its own unit (`systemd-run --user --unit=… -p MemoryMax=14G --setenv=LEANCHECKER_FLAGS=--fresh …`) so an out-of-memory kill elsewhere cannot stop it, or it stop the session.
- Do not edit the script while it runs: bash reads it as it goes.
