#!/usr/bin/env python3
"""The board's names side over a checkout, without a run: python3 -I tests/support/board_names.py <checkout>

For each board list whose tests are the tree's files, the names its runner judges that the list has no row for:
cases, interp, jvm (tests/cases), stdlib (tests/run_stdlib.sh's three greps), interop (tests/interop, its stub
aside), errors (tests/errors), modules (tests/modules' program directories) and pairs-twin (tests/support/
pairs_programs.py's case programs). Prints a line per list and, last, the lists that need a run with --update."""
import os
import re
import sys

root = os.path.abspath(sys.argv[1])


def files_names(d):
    out = set()
    for f in os.listdir(os.path.join(root, d)):
        p = os.path.join(root, d, f)
        if os.path.isdir(p):
            out.add(f)
        elif f.endswith(".scala"):
            out.add(f[:-6])
    return out


def rows(suite):
    p = os.path.join(root, "tests", "board", "suites", suite + ".tsv")
    if not os.path.exists(p):
        return set()
    return {l.split("\t")[0] for l in open(p, encoding="utf-8") if l.strip() and not l.startswith("#")}


def sources(name):
    p = os.path.join(root, "tests", "cases", name)
    if os.path.isdir(p):
        return [open(os.path.join(p, f), encoding="utf-8", errors="replace").read() for f in sorted(os.listdir(p)) if f.endswith(".scala")]
    return [open(p + ".scala", encoding="utf-8", errors="replace").read()]


def stdlib_runs(name):
    if not os.path.exists(os.path.join(root, "tests", "cases", name + ".expected")):
        return False
    return not any(re.search(r"scala\.scalajs|@js\b|@js\(|jsImport|jsExport|js\.Dynamic|js\.Array|js\.Object|import js\.", t)
                   or re.search(r"^// std: lean$", t, re.M) or re.search(r"^import sourcecode|sourcecode\.", t, re.M) for t in sources(name))


cases = files_names("tests/cases")
names = {
    "cases": cases, "interp": cases, "jvm": cases,
    "stdlib": {n for n in cases if stdlib_runs(n)},
    "interop": files_names("tests/interop") - {"scalajs-stub"},
    "errors": files_names("tests/errors"),
}
# tests/modules.sh's programs, its directories (its other units, own-* and split-*, are checks of no list row)
names["modules"] = {f for f in os.listdir(os.path.join(root, "tests", "modules")) if os.path.isdir(os.path.join(root, "tests", "modules", f))}
sys.path.insert(0, os.path.join(root, "tests", "support"))
sys.dont_write_bytecode = True
import pairs_programs  # noqa: E402
names["pairs-twin"] = {p[0] for p in pairs_programs.case_programs(root, ["js", "jvm"])}
need = []
for suite, ns in names.items():
    missing = sorted(ns - rows(suite))
    print(f"{suite}: {len(ns)} tests, {len(missing)} with no row" + (f": {' '.join(missing)}" if missing else ""))
    if missing:
        need.append(suite)
print("need a run with --update: " + (" ".join(need) if need else "none"))
