#!/usr/bin/env bash
# B-15: Moonkale's edit loop (moonkale-sources-sophia example edit_loop, Moonkale 26752448)
# against synthetic Sophia stores. Edit: the last integer literal + 1, or else the last hole ?(Int64) filled with 1.
set -euo pipefail
# Stores: sophia --store $W/genN.sophia init && sophia --store ... gen N (N = 1000, 10000, 100000).
# The edit loop: cd Moonkale (26752448+) && CARGO_TARGET_DIR=$HOME/.cache/sophia-corpora/moonkale-target cargo build --release -p moonkale-sources-sophia --example edit_loop
W="$HOME/.cache/sophia-corpora/b15"
SOPHIA=/home/daniel/Code/CategoryTheory/Sophia/target/release/sophia
LOOP="$HOME/.cache/sophia-corpora/moonkale-target/release/examples/edit_loop"
echo "== sample (in memory)"
"$LOOP" --runs 200 > "$W/sample.tsv"
for n in 1000 10000 100000; do
  cp "$W/gen$n.sophia" "$W/run$n.sophia"
  name=""
  # The most-referenced definitions first; take the first whose text has an integer literal.
  for cand in $("$SOPHIA" --store "$W/run$n.sophia" query 'select n, count(u) where refs(u, d) && name(n, d)' | sort -t$'\t' -k2 -n -r | head -40 | cut -f1); do
    "$SOPHIA" --store "$W/run$n.sophia" text "$cand" > "$W/orig$n.txt"
    if python3 -I -c '
import re,sys
s=open(sys.argv[1]).read()
ms=list(re.finditer(r"(?<![\w#])(\d+)(?![\w])", s))
if ms:
    m=ms[-1]
    out=s[:m.start()]+str(int(m.group(1))+1)+s[m.end():]
elif "?(Int64)" in s:
    i=s.rindex("?(Int64)")
    out=s[:i]+"1"+s[i+len("?(Int64)"):]
else:
    sys.exit(1)
open(sys.argv[2],"w").write(out)
' "$W/orig$n.txt" "$W/edit$n.txt"; then name=$cand; break; fi
  done
  deps=$("$SOPHIA" --store "$W/run$n.sophia" query 'select n, count(u) where refs(u, d) && name(n, d)' | grep -P "^$name\t" | cut -f2)
  echo "== $n definitions, $name ($deps direct dependants)"
  "$LOOP" --store "$W/run$n.sophia" --name "$name" --text "$W/edit$n.txt" --runs 200 > "$W/gen$n.tsv"
done
