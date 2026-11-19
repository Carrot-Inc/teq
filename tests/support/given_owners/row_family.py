"""row_family.py <generated dir> <errors dir> <row name> <note>: a generated grid's cells that are not whole as one
tests/errors program of the pending list (a row): every cell with a line not whole, or with a teq line scalac has not, copied as <owner>_k<NN>.scala, each
scalac 3.8.4 line of the cell (compare.json, column + 1) written after it as `// expect:`; the first file carries the
std flag and the count of scalac's lines; package_object's packages are renamed p -> q, beside named_package's."""
import json
import os
import re
import sys

G, E, name, note = sys.argv[1:5]
res = json.load(open(os.path.join(G, "compare.json")))
cells = {}
for owner, fam in res.items():
    for r in fam["rows"]:
        cells.setdefault((owner, r["file"]), []).append(r)
# a cell is the row's where a scalac line is not whole or teq prints a line scalac does not (teq-only)
extra = {(owner, x[0]) for owner, fam in res.items() for x in fam["teq-only"]}
pick = sorted(k for k, rows in cells.items() if any(r["status"] != "whole" for r in rows) or k in extra)
out = os.path.join(E, name)
os.makedirs(out, exist_ok=True)
total = sum(len(cells[k]) for k in pick)
for i, (owner, f) in enumerate(pick):
    text = open(os.path.join(G, owner, f)).read()
    if owner == "package_object":
        text = re.sub(r"\bp(\d\d)\b", r"q\1", text)
    target = f"{owner}_{f}"
    lines = [text.rstrip("\n")]
    if i == 0:
        lines.append(f"// {note}")
    lines.append("// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).")
    for r in cells[(owner, f)]:
        msg = re.sub(r"\bp(\d\d)\b", r"q\1", r["scalac"]) if owner == "package_object" else r["scalac"]
        lines.append(f"// expect: {target}:{r['line']}:{r['col']}: error: {msg}")
    if i == 0:
        lines += ["// teq: --std=scala-library", f"// expect: {total} errors found"]
    open(os.path.join(out, target), "w").write("\n".join(lines) + "\n")
print(len(pick), "cells,", total, "scalac lines in", out)
