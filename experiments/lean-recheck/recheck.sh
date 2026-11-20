#!/usr/bin/env bash
# Re-checks every Mathlib module with Lean's own kernel (`leanchecker`, shipped with the
# toolchain), one process per module, JOBS in parallel. Each module's declarations are
# replayed onto its imports; since every module is checked, all of Mathlib is covered
# (its dependencies Batteries, Aesop and Lean core are not). See recheck.md.
# Usage: recheck.sh OUTDIR [JOBS] [MODULE_FILTER_REGEX | @MODULE_LIST_FILE]
# LEANCHECKER_FLAGS (e.g. --fresh) is passed to every leanchecker call.
set -euo pipefail
out=${1:?OUTDIR}; jobs=${2:-6}; filter=${3:-.}
corp=${SOPHIA_CORPORA:-$HOME/.cache/sophia-corpora}
tc=$HOME/.elan/toolchains/leanprover--lean4---$(cut -d: -f2 "$corp/mathlib4/lean-toolchain")/bin
export PATH=$tc:$PATH
mkdir -p "$out"
cd "$corp/mathlib4"
if [[ $filter == @* ]]; then sort "${filter#@}" > "$out/modules.txt"
else find Mathlib -name '*.lean' | sed 's/\.lean$//; s|/|.|g' | grep -E "$filter" | sort > "$out/modules.txt"; fi
export LEANCHECKER_FLAGS=${LEANCHECKER_FLAGS:-}
echo "$(wc -l < "$out/modules.txt") modules, $jobs jobs, flags '$LEANCHECKER_FLAGS', $(date -Is)" > "$out/run.txt"
start=$(date +%s)
# One line per module: module, exit code, seconds, peak KB (from /usr/bin/time).
lake env bash -c '
  check() { /usr/bin/time -f "%e\t%M" -o "$2/time.$$" leanchecker $LEANCHECKER_FLAGS "$1" </dev/null >"$2/log.$$" 2>&1; local rc=$?
            printf "%s\t%s\t%s\n" "$1" "$rc" "$(tail -1 "$2/time.$$")"
            if [ "$rc" != 0 ]; then { echo "== $1"; cat "$2/log.$$"; } >> "$2/failures.txt"; fi; rm -f "$2/log.$$" "$2/time.$$"; }
  export -f check
  xargs -P "$1" -I{} bash -c "check {} \"$2\"" < "$2/modules.txt"' _ "$jobs" "$out" > "$out/results.tsv"
end=$(date +%s)
echo "wall $((end - start)) s, $(date -Is)" >> "$out/run.txt"
