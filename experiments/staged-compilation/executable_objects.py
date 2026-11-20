# E-7: reads a PJRT-serialised XLA CPU executable (Reactant 0.2.295's CPU
# build) as protobuf wire format just far enough to find its object files: one
# run of contiguous length-delimited entries, each {1: ELF object, 2: kernel
# name}. It sorts those entries (a canonical order) and compares every file
# with the first: whole-file digest, canonical digest, and the bytes that still
# differ after sorting, grouped by the byte before them (on these files a
# protobuf tag, checked by hand against Reactant's schema; not in general).
# See lux_train_step.md.
#   python3 -I executable_objects.py EXECUTABLE...
import hashlib, sys
from collections import Counter

def varint(b, i):
    n = s = 0
    while True:
        c = b[i]; i += 1
        n |= (c & 0x7F) << s; s += 7
        if c < 0x80:
            return n, i

def entry_at(b, i):
    """A length-delimited field at i whose payload is {1: ELF, 2: name}: (end, bytes) or None."""
    tag, j = varint(b, i)
    if tag & 7 != 2:
        return None
    n, k = varint(b, j)
    p = b[k:k + n]
    if len(p) < n or not p.startswith(b"\n"):
        return None
    m, q = varint(p, 1)
    if p[q:q + 4] != b"\x7fELF" or q + m >= len(p) or p[q + m] != 0x12:
        return None
    l, r = varint(p, q + m + 1)
    return (k + n, b[i:k + n]) if r + l == len(p) else None

def canonical(b):
    first = b.find(b"\x7fELF")
    start = next(s for s in range(first - 1, max(first - 16, 0), -1) if entry_at(b, s))
    entries, i = [], start
    while (e := entry_at(b, i)) is not None:
        i, raw = e
        entries.append(raw)
    assert b.find(b"\x7fELF", i) < 0, "an object outside the run of entries"
    assert len({e[0] for e in entries}) == 1, "entries with different field tags; sorting would mix fields"
    return b[:start] + b"".join(sorted(entries)) + b[i:], len(entries)

h = lambda x: hashlib.sha256(x).hexdigest()[:16]
ref = None
for path in sys.argv[1:]:
    b = open(path, "rb").read()
    c, n = canonical(b)
    line = f"{path}: {len(b)} B, {n} objects; whole {h(b)}, entries sorted {h(c)}"
    if ref is None:
        ref = c
    elif len(c) != len(ref):
        line += "; length differs from the first"
    else:
        d = [i for i in range(len(c)) if c[i] != ref[i]]
        kinds = Counter((c[i - 1], ref[i], c[i]) for i in d)
        line += f"; vs first: {len(d)} bytes differ" + ("".join(
            f", {k} x (after byte 0x{t:02x}: {x} -> {y})" for (t, x, y), k in kinds.items()) if d else "")
    print(line)
