#!/usr/bin/env python3
"""The stand-in of bench/pairs.py tests/gate-scripts.sh puts in its landing: a line's comparison that measures
nothing, its runs and line files written where bench/pairs.py writes them, and a line naming the two binaries;
inconclusive (exit 3) for a line bench/pairs-stand-in.txt names; for a line it names as `die:<line>`, the comparison's
job killed (the process out/compare/pid names), as a job killed for memory is, before the line answers."""
import json, os, sys

a = sys.argv
at = a.index("--out")
out, label = a[at + 1], a[a.index("--label") + 1]
os.makedirs(out, exist_ok=True)
open(os.path.join(out, "runs.jsonl"), "w").close()
json.dump({"label": label}, open(os.path.join(out, "line.json"), "w"))
words = open("bench/pairs-stand-in.txt").read().split() if os.path.exists("bench/pairs-stand-in.txt") else []
if f"die:{label}" in words:
    import signal
    os.kill(int(open("out/compare/pid").read()), signal.SIGKILL)
inconclusive = label in words
print(f"{label}: the stand-in of bench/pairs.py, {a[1]} of {a[at - 2]} and {a[at - 1]}" + (", 1 inconclusive" if inconclusive else ""))
sys.exit(3 if inconclusive else 0)
