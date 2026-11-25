#!/usr/bin/env python3
"""The plan of tests/new-cases.sh: the programs a change adds over a base and how each suite of the gate that makes
tests out of them runs on them. python3 -I tests/support/new_cases.py <base>   (from the tree's root)

The new programs: what `git diff --no-renames --diff-filter=A <base>...HEAD` adds (a program moved in is new) under tests/cases (a case, <name>.scala or a
directory), tests/errors (a program of the errors suite) and tests/modules (a program's directory), each still in the
tree. A line a suite, tab-separated: the suite, `named`, `whole` or `none`, and its arguments (named: the names; whole:
nothing) or, for `none`, the rule that leaves it unreached. The fifteen suites, always in this order:
  cases interp jvm jvm-21 stdlib split   named by TEQ_CASES (tests/support/cases.sh), where a new case is one it runs
                                         (stdlib: its three greps; split: an .expected, not the JVM's platform, not
                                         listed in tests/pending/cases.txt)
  errors                                 named by TEQ_ERRORS, where an errors program is new
  pairs-twin                             named (cases/<name>), the cases tests/support/pairs_programs.py gives (no jars)
  modules                                named (tests/modules.sh's positional names), where a module is new
  tasty-exec                             named, the new cases tests/tasty/exec/closure.txt holds
  workers                                whole, where a new case or module matches *export* (its export checks; a
                                         suite of checks named by their failures alone, so no part of it is held)
  split-watch                            whole, where a new case is one it builds (an .expected, not in
                                         tests/pending/cases.txt, no `//> using file`, a `// std:` line naming lean
                                         if any, no `// split-watch: jvm`), the same, held whole alone
  jvm-watch                              whole, where a new case's row in tests/board/suites/jvm.tsv reads pass
                                         (its units are that list's passes)
  analysis-bytes                         whole, where a new module is one it builds (not in tests/pending/modules.txt)
  cargo-test                             none: it reads the whole tree whatever is new; the gate runs it
"""
import os
import re
import subprocess
import sys

SUITES = ["cases", "interp", "jvm", "jvm-21", "stdlib", "split", "errors", "pairs-twin", "modules", "tasty-exec",
          "workers", "split-watch", "jvm-watch", "analysis-bytes", "cargo-test"]


def added(base):
    out = subprocess.run(["git", "diff", "--no-renames", "--diff-filter=A", "--name-only", f"{base}...HEAD", "--", "tests/cases",
                          "tests/errors", "tests/modules"], capture_output=True, text=True)
    if out.returncode != 0:
        print(f"new-cases: no diff over {base}: {out.stderr.strip()}", file=sys.stderr)
        sys.exit(2)
    cases, errors, modules = set(), set(), set()
    for path in out.stdout.split():
        parts = path.split("/")
        if len(parts) < 3:
            continue
        top, first = parts[1], parts[2]
        if top == "modules":
            if len(parts) > 3 and os.path.isdir(os.path.join("tests", "modules", first)):
                modules.add(first)
            continue
        name = first[:-len(".scala")] if len(parts) == 3 and first.endswith(".scala") else first if len(parts) > 3 else None
        if not name:
            continue
        here = os.path.exists(os.path.join("tests", top, name + ".scala")) or os.path.isdir(os.path.join("tests", top, name))
        if here:
            (cases if top == "cases" else errors).add(name)
    return sorted(cases), sorted(errors), sorted(modules)


def sources(name):
    p = os.path.join("tests", "cases", name)
    if os.path.isdir(p):
        return [open(os.path.join(p, f), encoding="utf-8", errors="replace").read() for f in sorted(os.listdir(p)) if f.endswith(".scala")]
    return [open(p + ".scala", encoding="utf-8", errors="replace").read()]


def listed(path):
    if not os.path.exists(path):
        return set()
    return {l.split("\t")[0] for l in open(path, encoding="utf-8") if l.strip() and not l.startswith("#")}


def any_line(name, rx):
    return any(re.search(rx, t, re.M) for t in sources(name))


def plan(base):
    cases, errors, modules = added(base)
    pending_cases = listed("tests/pending/cases.txt")
    expected = lambda n: os.path.exists(os.path.join("tests", "cases", n + ".expected"))
    jvm_platform = lambda n: any_line(n, r"^//> using platform jvm")
    stdlib = [n for n in cases if expected(n) and not any_line(n, r"scala\.scalajs|@js\b|@js\(|jsImport|jsExport|js\.Dynamic|js\.Array|js\.Object|import js\.")
              and not any_line(n, r"^// std: lean$") and not any_line(n, r"^import sourcecode|sourcecode\.")]
    split = [n for n in cases if expected(n) and not jvm_platform(n) and n not in pending_cases]
    sys.path.insert(0, os.path.join("tests", "support"))
    sys.dont_write_bytecode = True
    import pairs_programs
    pairs = sorted({p[0] for p in pairs_programs.case_programs(".", ["js", "jvm"])} & {f"cases/{n}" for n in cases})
    closure = os.path.join("tests", "tasty", "exec", "closure.txt")
    held = {l.split()[0] for l in open(closure, encoding="utf-8") if l.strip() and not l.startswith("#")} if os.path.exists(closure) else set()
    tasty = [n for n in cases if n in held]
    export = [n for n in cases + modules if "export" in n]
    def std_modes(n):
        found = [m.group(1) for t in sources(n) for m in [re.search(r"^// std: (.*)", t, re.M)] if m]
        return found[0].split() if found else []
    watch = [n for n in cases if expected(n) and n not in pending_cases and not any_line(n, r"^//> using files? ")
             and not (std_modes(n) and "lean" not in std_modes(n)) and not any_line(n, r"^// split-watch: jvm")]
    jvm_rows = {}
    if os.path.exists("tests/board/suites/jvm.tsv"):
        for l in open("tests/board/suites/jvm.tsv", encoding="utf-8"):
            if l.strip() and not l.startswith("#"):
                p = l.split("\t")
                jvm_rows[p[0]] = p[1] if len(p) > 1 else ""
    jvm_pass = [n for n in cases if jvm_rows.get(n) == "pass"]
    pending_modules = listed("tests/pending/modules.txt")
    built = [m for m in modules if m not in pending_modules]
    none = lambda why: ("none", why)
    rows = {}
    for s in ("cases", "interp", "jvm", "jvm-21"):
        rows[s] = ("named", " ".join(cases)) if cases else none("no new case")
    rows["stdlib"] = ("named", " ".join(stdlib)) if stdlib else none("no new case it runs (an .expected, no JavaScript interop, no `// std: lean`, no sourcecode)" if cases else "no new case")
    rows["split"] = ("named", " ".join(split)) if split else none("no new case it runs (an .expected, not the JVM's platform, not in tests/pending/cases.txt)" if cases else "no new case")
    rows["errors"] = ("named", " ".join(errors)) if errors else none("no new program of tests/errors")
    rows["pairs-twin"] = ("named", ",".join(pairs)) if pairs else none("no new case it builds (none without jars)" if cases else "no new case")
    rows["modules"] = ("named", " ".join(modules)) if modules else none("no new module")
    rows["tasty-exec"] = ("named", " ".join(tasty)) if tasty else none("no new case in tests/tasty/exec/closure.txt")
    rows["workers"] = ("whole", "") if export else none("no new case or module matches *export*")
    rows["split-watch"] = ("whole", "") if watch else none("no new case it takes" if cases else "no new case")
    rows["jvm-watch"] = ("whole", "") if jvm_pass else none("no new case's row in tests/board/suites/jvm.tsv reads pass" if cases else "no new case")
    rows["analysis-bytes"] = ("whole", "") if built else none("no new module it builds" if modules else "no new module")
    rows["cargo-test"] = none("it reads the whole tree whatever is new: the gate runs it")
    return (cases, errors, modules), [(s,) + rows[s] for s in SUITES]


def main(argv):
    if len(argv) != 1:
        print(__doc__, file=sys.stderr)
        return 2
    (cases, errors, modules), rows = plan(argv[0])
    print(f"#\tnew programs over {argv[0]}: {len(cases)} cases, {len(errors)} errors programs, {len(modules)} modules")
    for s, how, what in rows:
        print(f"{s}\t{how}\t{what}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
