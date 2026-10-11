#!/bin/bash
# Whether the GitHub gate (.github/workflows/gate.yml) runs every suite tests/all.sh names: the suites its jobs
# select (a matrix row's `suites:` line, a step's `tests/all.sh <suites>`) and the ones its `# excluded: <suite>
# <reason>` lines leave out, against the names of tests/all.sh's `suite` lines. A suite of tests/all.sh neither
# selected nor excluded fails, and so does a name either side lacks, a suite selected twice or selected and
# excluded, and an exclusion without its reason. Reads the two files alone.
cd "$(dirname "$0")/.." || exit 1
python3 - tests/all.sh .github/workflows/gate.yml <<'PY'
import re, sys

all_sh, workflow = (open(p, encoding="utf-8").read() for p in sys.argv[1:3])
suites = []
for m in re.finditer(r"^suite\s+(\S+)", all_sh, re.M):
    if m.group(1) not in suites:
        suites.append(m.group(1))
selected = []
for m in re.finditer(r"^[ \t]+suites:[ \t]*(.*)$", workflow, re.M):
    selected += m.group(1).split()
for m in re.finditer(r"^\s*(?:-\s+run:\s*)?tests/all\.sh[ \t]+(.*)$", workflow, re.M):
    words = m.group(1).split()
    if words and not words[0].startswith("${{"):
        selected += words
excluded = {}
fails = []
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
for f in fails:
    print(f"FAIL {f}")
print(f"gate-coverage: {len(suites)} suites of tests/all.sh, {len(set(selected) & set(suites))} run by the workflow, {len(excluded)} excluded, {len(fails)} failed")
sys.exit(1 if fails else 0)
PY
