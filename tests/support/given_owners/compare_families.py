"""compare_families.py <dir>: for each family <dir>/<family>/ with <dir>/<family>.scalac.txt and .teq.txt, scalac
3.8.4's diagnostics (each box's code, file, line, column + 1, the message's first line) against teq's at the same file,
line and column: `whole` (the line is scalac's), `place` (scalac's words at another place of that file and line), or
`differ`; and teq's diagnostics at no scalac place. Writes <dir>/compare.json and prints every line not whole."""
import json
import os
import re
import sys

F = sys.argv[1]
res, totals = {}, {"whole": 0, "place": 0, "differ": 0, "teq-only": 0}
for fam in sorted(d for d in os.listdir(F) if os.path.isdir(os.path.join(F, d))):
    sc = re.sub(r"\x1b\[[0-9;]*m", "", open(f"{F}/{fam}.scalac.txt").read()).split("\n")
    S, i = [], 0
    while i < len(sc):
        m = re.match(r"^-- \[(E\d+)\] .*?: (.*?):(\d+):(\d+)\s*-*\s*$", sc[i])
        if not m:
            i += 1
            continue
        code, path, l, c = m.groups()
        j, msg = i + 1, []
        while j < len(sc) and not sc[j].startswith("-- ["):
            t = re.match(r"^\s*\|(.*)$", sc[j])
            if t:
                msg.append(t.group(1))
            j += 1
        k = next((x for x, v in enumerate(msg) if re.match(r"^\s*\^+\s*$", v)), -1)
        body = [v.strip() for v in msg[k + 1:]]
        S.append((code, os.path.basename(path), int(l), int(c) + 1, body[0] if body else ""))
        i = j
    T = []
    for l in open(f"{F}/{fam}.teq.txt"):
        m = re.match(r"^(.*?):(\d+):(\d+): error: (.*)$", l.rstrip("\n"))
        if m:
            T.append((os.path.basename(m.group(1)), int(m.group(2)), int(m.group(3)), m.group(4)))
    rows, used = [], set()
    for code, f, l, c, msg in S:
        at = [k for k, x in enumerate(T) if x[:3] == (f, l, c)]
        if at and T[at[0]][3] == msg:
            status, k = "whole", at[0]
        else:
            near = [k for k, x in enumerate(T) if x[0] == f and x[3] == msg and k not in used]
            status, k = ("place", near[0]) if near else ("differ", at[0] if at else None)
        if k is not None:
            used.add(k)
        rows.append({"code": code, "file": f, "line": l, "col": c, "scalac": msg, "teq": T[k] if k is not None else None, "status": status})
        totals[status] += 1
    extra = [x for k, x in enumerate(T) if k not in used]
    totals["teq-only"] += len(extra)
    res[fam] = {"rows": rows, "teq-only": extra}
    print(f"== {fam}: {sum(r['status'] == 'whole' for r in rows)} whole of {len(rows)}")
    for r in rows:
        if r["status"] != "whole":
            print(f"   {r['status']} {r['code']} {r['file']}:{r['line']}:{r['col']} | S: {r['scalac'][:110]} | T: {(r['teq'] or ('', 0, 0, ''))[3][:110]}")
    for x in extra:
        print(f"   teq-only {x[0]}:{x[1]}:{x[2]} {x[3][:120]}")
json.dump(res, open(f"{F}/compare.json", "w"), indent=1)
print(totals)
