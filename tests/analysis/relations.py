#!/usr/bin/env python3
"""Checks tests/analysis/relations.txt against the dumps of a run of tests/analysis.sh.

  relations.py <relations.txt> <results dir>

Each line `same|differs <p> <q> <class> <name>` holds when zinc's name hash of <name> in
<class> (scope Default) is equal, or unequal, in projects <p> and <q>, on scalac's side and on
teq's. A project without its dump, or a class or name without its hash, fails."""

import json
import os
import sys


def name_hash(results, project, side, cls, name):
    path = os.path.join(results, "%s.%s.json" % (project, side))
    if not os.path.exists(path):
        return None, "no %s" % path
    for h in json.load(open(path)).get("hashes", []):
        if h.get("name") == cls:
            for n, scope, v in h.get("nameHashes", []):
                if n == name and scope == "Default":
                    return v, None
            return None, "%s has no hash of %s in %s" % (cls, name, path)
    return None, "no class %s in %s" % (cls, path)


def main():
    relations, results = sys.argv[1], sys.argv[2]
    passed = failed = 0
    for line in open(relations):
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        kind, p, q, cls, name = line.split()
        for side in ("scalac", "teq"):
            a, why_a = name_hash(results, p, side, cls, name)
            b, why_b = name_hash(results, q, side, cls, name)
            if why_a or why_b:
                print("FAIL    %s (%s): %s" % (line, side, why_a or why_b))
                failed += 1
            elif (a == b) != (kind == "same"):
                print("FAIL    %s (%s): %s and %s" % (line, side, a, b))
                failed += 1
            else:
                passed += 1
    print("relations: %d passed, %d failed" % (passed, failed))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
