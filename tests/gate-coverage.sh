#!/bin/bash
# Whether the GitHub gate (.github/workflows/gate.yml) runs every suite tests/all.sh names: the suites its jobs
# select (a matrix row's `suites:` line, a step's `tests/all.sh <suites>`; a suite as pieces, `modules.1/2`, each of
# 1..n once over the jobs) and the ones its `# excluded: <suite>
# <reason>` lines leave out, against the names of tests/all.sh's `suite` lines. A suite of tests/all.sh neither
# selected nor excluded fails, and so does a name either side lacks, a suite selected twice or selected and
# excluded, and an exclusion without its reason. And the same of the landing gate, tests/gate.sh: its lines (the
# machines' suites, the comparison's and the local ones) and its own `# excluded: <suite> <reason>` lines against
# tests/all.sh's suites. Reads the three files alone.
cd "$(dirname "$0")/.." || exit 1
python3 - tests/all.sh .github/workflows/gate.yml tests/gate.sh <<'PY'
import re, sys

all_sh, workflow, gate = (open(p, encoding="utf-8").read() for p in sys.argv[1:4])
suites = []
for m in re.finditer(r"^suite\s+(\S+)", all_sh, re.M):
    if m.group(1) not in suites:
        suites.append(m.group(1))
selected = []
# A suite run as pieces (`modules.1/2`, tests/all.sh's piece names) counts as selected once when each of 1..n is
# named exactly once over the jobs.
pieces = {}
def select(words):
    for w in words:
        m = re.fullmatch(r"([a-z0-9-]+)\.(\d+)/(\d+)", w)
        if m:
            pieces.setdefault(m.group(1), []).append((int(m.group(2)), int(m.group(3))))
        else:
            selected.append(w)
for m in re.finditer(r"^[ \t]+suites:[ \t]*(.*)$", workflow, re.M):
    select(m.group(1).split())
for m in re.finditer(r"^\s*(?:-\s+run:\s*)?tests/all\.sh[ \t]+(.*)$", workflow, re.M):
    words = m.group(1).split()
    if words and not words[0].startswith("${{"):
        select(words)
excluded = {}
fails = []
for name, ps in pieces.items():
    n = ps[0][1]
    if {m for _, m in ps} != {n} or sorted(k for k, _ in ps) != list(range(1, n + 1)):
        fails.append(f"{name}: its pieces are {', '.join(f'{k}/{m}' for k, m in ps)}, not each of 1..n once")
    selected.append(name)
for m in re.finditer(r"^#\s*excluded:\s*(\S+)[ \t]*(.*)$", workflow, re.M):
    if not m.group(2).strip():
        fails.append(f"{m.group(1)} is excluded without a reason")
    excluded[m.group(1)] = m.group(2).strip()
for name in suites:
    n = selected.count(name)
    if n == 0 and name not in excluded:
        fails.append(f"{name}: neither run nor excluded")
    elif n > 1:
        fails.append(f"{name}: selected {n} times")
    elif n == 1 and name in excluded:
        fails.append(f"{name}: selected and excluded")
for name in sorted(set(selected) | set(excluded)):
    if name not in suites:
        fails.append(f"{name}: no suite of tests/all.sh")
# The landing gate's lines: the machines' suites (`  "<name> <command>"` in its `machine` list), the comparison's
# and the local lines (`compare="..."`, `local_lines="..."`).
listed = re.search(r"^machine=\(\n(.*?)^\)", gate, re.M | re.S)
lines = re.findall(r'^\s*"(\S+) ', listed.group(1), re.M) if listed else []
for var in ("compare", "local_lines"):
    m = re.search(rf'^{var}="([^"]*)"', gate, re.M)
    lines += m.group(1).split() if m else []
gate_excluded = {}
for m in re.finditer(r"^#\s*excluded:\s*(\S+)[ \t]*(.*)$", gate, re.M):
    if not m.group(2).strip():
        fails.append(f"tests/gate.sh: {m.group(1)} is excluded without a reason")
    gate_excluded[m.group(1)] = m.group(2).strip()
for name in suites:
    n = lines.count(name)
    if n == 0 and name not in gate_excluded:
        fails.append(f"tests/gate.sh: {name} neither a line nor excluded")
    elif n > 1:
        fails.append(f"tests/gate.sh: {name} a line {n} times")
    elif n == 1 and name in gate_excluded:
        fails.append(f"tests/gate.sh: {name} a line and excluded")
for name in sorted(gate_excluded):
    if name not in suites:
        fails.append(f"tests/gate.sh: {name} is excluded and no suite of tests/all.sh")
for f in fails:
    print(f"FAIL {f}")
print(f"gate-coverage: {len(suites)} suites of tests/all.sh, {len(set(selected) & set(suites))} run by the workflow, {len(excluded)} excluded; {len(set(lines) & set(suites))} lines of tests/gate.sh, {len(gate_excluded)} excluded; {len(fails)} failed")
sys.exit(1 if fails else 0)
PY
