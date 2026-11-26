#!/usr/bin/env python3
"""The stale entries of the lists of tests, without a run: python3 -I tests/support/stale_entries.py [<suite>...]

A list of the tests expected to fail (tests/pending/<suite>.txt) and a board list (tests/board/suites/<suite>.tsv)
name tests; an entry is stale when its suite has no such test, a deleted test's entry, which a piece-wise --update
keeps (tests/support/pending.py). Each list is read against its suite's inventory, the names its runner judges,
computed from the tree alone (its files, or a fixed table the run itself holds equal); every stale name is printed
with its list and the command that takes it out. A list whose names exist only when its suite runs (the checks of a
suite that names its failures alone, a generated set) is said as not checked here: its run holds it
(tests/support/pending.py fails a run whose list names a test it did not judge). A dotty list is read against the
pinned scala3 checkout where it is present (tests/board/dotty.py's corpus), else said as not checked.

The last line names every list with a stale entry, or none; the exit status is 1 when one has, 0 when none, 2 for a
command it cannot read. --self-test runs the checker over planted trees (a stale name in a list, in a board list, in the
assertions row's list; interop's checks, which are no files, not stale).
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def entries(dirpath, keep=lambda name, isdir: True):
    """<name> for each <name>.scala file and each directory in dirpath (sorted), as the runners walk them."""
    out = set()
    if not os.path.isdir(dirpath):
        return out
    for f in os.listdir(dirpath):
        p = os.path.join(dirpath, f)
        if os.path.isdir(p):
            if keep(f, True):
                out.add(f)
        elif f.endswith(".scala") and keep(f[:-6], False):
            out.add(f[:-6])
    return out


def texts(root, d, name):
    p = os.path.join(root, d, name)
    files = [os.path.join(p, f) for f in sorted(os.listdir(p)) if f.endswith(".scala")] if os.path.isdir(p) else [p + ".scala"]
    return [open(f, encoding="utf-8", errors="replace").read() for f in files if os.path.exists(f)]


STDLIB_OUT = re.compile(r"scala\.scalajs|@js\b|@js\(|jsImport|jsExport|js\.Dynamic|js\.Array|js\.Object|import js\.")


def inventories(root):
    """suite -> (its names, the command that takes a stale entry out) for every list this checks from the tree."""
    cases = entries(os.path.join(root, "tests", "cases"))
    inv = {}
    for s in ("cases", "interp", "jvm"):
        inv[s] = (cases, {"cases": "tests/run.sh --update", "interp": "tests/run_interp.sh --update", "jvm": "tests/run_jvm.sh --update"}[s])
    # the assertions row builds every case; its list is under --fails-only, where a name no run judges is kept, so the
    # list's line goes by hand and the board's row with the next --pending-update
    inv["jvm-assertions"] = (cases, "by hand from tests/pending/jvm-assertions.txt, then tests/jvm-assertions.sh --pending-update")
    inv["pending-cases"] = (entries(os.path.join(root, "tests", "pending-cases")), "tests/run.sh --dir=tests/pending-cases --update")

    def stdlib_runs(n):
        if not os.path.exists(os.path.join(root, "tests", "cases", n + ".expected")):
            return False
        return not any(STDLIB_OUT.search(t) or re.search(r"^// std: lean$", t, re.M) or re.search(r"^import sourcecode|sourcecode\.", t, re.M)
                       for t in texts(root, os.path.join("tests", "cases"), n))
    inv["stdlib"] = ({n for n in cases if stdlib_runs(n)}, "tests/run_stdlib.sh --update")
    errors = entries(os.path.join(root, "tests", "errors"))
    inv["errors"] = (errors | {n + ".check" for n in errors if os.path.exists(os.path.join(root, "tests", "errors", n + ".check"))},
                     "tests/run_errors.sh --update")
    # interop: its programs, and the checks tests/run_interop.sh names: each `expect "<check>" ...`, and each
    # `check ... "$name <words>"` of a `for name in <names>` loop
    interop = entries(os.path.join(root, "tests", "interop")) - {"scalajs-stub"}
    script = os.path.join(root, "tests", "run_interop.sh")
    checks = set()
    if os.path.exists(script):
        text = open(script, encoding="utf-8").read()
        checks = set(re.findall(r'^\s*expect "([^"$]+)"', text, re.M))
        for loop, body in re.findall(r'^for name in ([\w ]+); do\n((?:.*\n)*?)done', text, re.M):
            for words in re.findall(r'\bcheck [^\n]*"\$name ([^"$]+)"', body):
                checks |= {f"{n} {words}" for n in loop.split()}
    inv["interop"] = (interop | checks, "tests/run_interop.sh --update")
    split = entries(os.path.join(root, "tests", "split"),
                    lambda n, isdir: isdir and os.path.exists(os.path.join(root, "tests", "split", n + ".expected"))
                    and not any(re.search(r"^//> using platform jvm", t, re.M) for t in texts(root, os.path.join("tests", "split"), n)))
    inv["split"] = (split, "tests/split.sh --update")
    inv["modules"] = (entries(os.path.join(root, "tests", "modules"), lambda n, isdir: isdir), "tests/modules.sh --update")
    cp = os.path.join(root, "tests", "classpath")
    inv["classpath"] = (entries(cp, lambda n, isdir: not isdir and n not in ("jvm", "js"))
                        | {"jvm/" + n for n in entries(os.path.join(cp, "jvm"))} | {"js/" + n for n in entries(os.path.join(cp, "js"))},
                        "tests/classpath.sh --update")
    programs = entries(os.path.join(root, "tests", "init-view", "programs"), lambda n, isdir: isdir)
    for s in ("init-view", "init-view-full", "init-view-scalac"):
        inv[s] = (programs, "tests/init-view.sh --update")
    inv["parser-fixtures"] = (entries(os.path.join(root, "tests", "parser", "fixtures")), "tests/parser.sh --update")
    inv["jvm-abi"] = (entries(os.path.join(root, "tests", "support", "abi"), lambda n, isdir: not isdir), "tests/run_jvm.sh --update")
    sys.path.insert(0, os.path.join(root, "tests", "support"))
    sys.dont_write_bytecode = True
    try:
        import pairs_programs
        inv["pairs-twin"] = ({p[0] for p in pairs_programs.case_programs(root, ["js", "jvm"])}, "tests/pairs-twin.sh --update")
    except (ImportError, OSError):
        pass
    cells = os.path.join(root, "tests", "java-lang-names", "cells.tsv")
    if os.path.exists(cells):
        names = set()
        for l in open(cells, encoding="utf-8"):
            p = l.rstrip("\n").split("\t")
            if l.strip() and not l.startswith("#") and len(p) > 1:
                names.add(f"{p[0]}__{p[1]}")
        inv["java-lang-names"] = (names, "tests/java-lang-names.sh --update")
    # conversion-matrix-<set>: the ids `<set>/<name>` of scalac's record, which its check holds the generated cells to
    record = os.path.join(root, "tests", "conversion-matrix", "scalac.jsonl")
    if os.path.exists(record):
        import json
        sets = {}
        for l in open(record, encoding="utf-8"):
            if l.strip():
                set_, _, name = json.loads(l)["id"].partition("/")
                sets.setdefault(set_, set()).add(name)
        for set_, names in sets.items():
            inv["conversion-matrix-" + set_] = (names, "python3 tests/conversion-matrix/run.py --whole --update")
    kept = os.path.join(root, "tests", "kept-body-lists", "expected.tsv")
    if os.path.exists(kept):
        progs = {l.split("\t")[0] for l in open(kept, encoding="utf-8") if l.strip() and not l.startswith("#")}
        inv["kept-body-lists"] = ({f"{p}/{leg}" for p in progs for leg in ("js", "interp")}, "python3 tests/kept-body-lists/run.py --update")
    return inv


# The lists whose names exist only when their suite runs: their run holds them.
AT_RUN = {
    "tasty": "a suite that names its failures alone (its run fails a listed check no line names)",
    "workers": "a suite that names its failures alone",
    "split-watch": "a suite that names its failures alone",
    "std-kinds": "names from the std's listing, compared by its run",
    "constants": "its cells from its generator, read by its run",
    "interp-script": "its runs from its scripts, judged by its run",
    "branch-scripts": "its scripts, judged by its run",
    "scala3-run": "its tests from $SCALA3, a checkout not pinned to a commit",
    "scala3-run-interp": "its tests from $SCALA3, a checkout not pinned to a commit",
    "scala3-macros": "its tests from $SCALA3, a checkout not pinned to a commit",
    "scala3-macros-interp": "its tests from $SCALA3, a checkout not pinned to a commit",
}


def listed(root, suite):
    p = os.path.join(root, "tests", "pending", suite + ".txt")
    if not os.path.exists(p):
        return set()
    return {l.split("\t")[0].rstrip("\n") for l in open(p, encoding="utf-8") if l.strip() and not l.startswith("#")}


def rows(root, suite, board_dir=("tests", "board", "suites")):
    p = os.path.join(root, *board_dir, suite + ".tsv")
    if not os.path.exists(p):
        return set()
    return {l.split("\t")[0] for l in open(p, encoding="utf-8") if l.strip() and not l.startswith("#")}


def dotty_inventories(root):
    """dotty-<row> -> its test names, from the pinned scala3 checkout (tests/board/dotty.py), or None without it."""
    sys.path.insert(0, os.path.join(root, "tests", "board"))
    try:
        import dotty
        corpus = dotty.corpus()
        return {f"dotty-{row.name}": {t.name for t in dotty.tests_of(row, corpus)} for row in dotty.ROWS.rows()}
    except BaseException as e:  # any want of the checkout (dotty.py exits on one): said as not checked
        return (str(e) or type(e).__name__).split("\n")[0][:160]


def check(root, only=None):
    """The stale entries: lines to print and whether any list has one."""
    out, stale_lists = [], []
    inv = inventories(root)
    suites = sorted({f[:-4] for f in os.listdir(os.path.join(root, "tests", "pending")) if f.endswith(".txt")}
                    | {f[:-4] for f in os.listdir(os.path.join(root, "tests", "board", "suites")) if f.endswith(".tsv")})
    if only:
        suites = [s for s in suites if s in only]
    dotty = None
    for suite in suites:
        if suite in inv:
            names, command = inv[suite]
            for what, have in (("tests/pending/" + suite + ".txt", listed(root, suite)), ("tests/board/suites/" + suite + ".tsv", rows(root, suite))):
                gone = sorted(have - names)
                if gone:
                    out.append(f"stale: {what}: {len(gone)} of no test of {suite}'s: {' '.join(gone[:20])}{' ...' if len(gone) > 20 else ''} (take them out: {command})")
                    stale_lists.append(what)
        elif suite in AT_RUN:
            out.append(f"not checked here: {suite}: {AT_RUN[suite]}")
        elif suite.startswith("dotty-"):
            if dotty is None:
                dotty = dotty_inventories(root)
            if isinstance(dotty, str):
                out.append(f"not checked here: {suite}: no pinned scala3 checkout ({dotty})")
                continue
            names = dotty.get(suite)
            if names is None:
                out.append(f"not checked here: {suite}: no row of tests/board/rows.py by that name")
                continue
            have = listed(root, suite) | rows(root, suite[len("dotty-"):], ("tests", "board", "dotty"))
            gone = sorted(have - names)
            if gone:
                out.append(f"stale: {suite}: {len(gone)} of no test of its row's: {' '.join(gone[:20])} (take them out: python3 tests/board/dotty.py --only {suite[len('dotty-'):]} --update)")
                stale_lists.append(suite)
        else:
            out.append(f"not checked here: {suite}: no inventory of its names from the tree")
    out.append("stale entries: " + (" ".join(stale_lists) if stale_lists else "none"))
    return out, bool(stale_lists)


def self_test():
    import tempfile
    failures = []
    with tempfile.TemporaryDirectory() as d:
        for sub in ("tests/cases/dir_case", "tests/pending", "tests/board/suites", "tests/interop", "tests/errors"):
            os.makedirs(os.path.join(d, sub), exist_ok=True)
        open(os.path.join(d, "tests/cases/a.scala"), "w").write("@main def m() = println(1)\n")
        open(os.path.join(d, "tests/cases/dir_case/Main.scala"), "w").write("@main def m() = println(1)\n")
        open(os.path.join(d, "tests/interop/one.scala"), "w").write("object One\n")
        open(os.path.join(d, "tests/run_interop.sh"), "w").write('for name in imports jsvalues; do\n  check "$dir/expected" "$dir/actual" "$name with the default output path"\n'
                                                                  'done\nexpect "first line of imports" "$(head -1 x)" y\n')
        open(os.path.join(d, "tests/pending/cases.txt"), "w").write("a\tits note\ngone_case\tits note\n")
        open(os.path.join(d, "tests/board/suites/interp.tsv"), "w").write("# test\tverdict\na\tpass\t-\t\ndir_case\tpass\t-\t\nalso_gone\tfail\t-\tx\n")
        open(os.path.join(d, "tests/board/suites/interop.tsv"), "w").write("# test\tverdict\none\tpass\t-\t\nimports with the default output path\tpass\t-\t\n"
                                                                            "first line of imports\tpass\t-\t\n")
        open(os.path.join(d, "tests/pending/tasty.txt"), "w").write("some check\tnote\n")
        open(os.path.join(d, "tests/pending/jvm-assertions.txt"), "w").write("dir_case\tabstractness-changed\ngone_panic\tabstractness-changed\n")
        out, stale = check(d)
        text = "\n".join(out)
        if not stale:
            failures.append("a planted stale name read as none")
        if "stale: tests/pending/cases.txt: 1 of no test of cases's: gone_case (take them out: tests/run.sh --update)" not in out:
            failures.append(f"the pending list's stale name: {text}")
        if "stale: tests/board/suites/interp.tsv: 1 of no test of interp's: also_gone (take them out: tests/run_interp.sh --update)" not in out:
            failures.append(f"the board list's stale row: {text}")
        if ("stale: tests/pending/jvm-assertions.txt: 1 of no test of jvm-assertions's: gone_panic (take them out: by hand from "
                "tests/pending/jvm-assertions.txt, then tests/jvm-assertions.sh --pending-update)") not in out:
            failures.append(f"the assertions row's stale name: {text}")
        if "interop" in " ".join(l for l in out if l.startswith("stale:")):
            failures.append(f"interop's check, which is no file, read as stale: {text}")
        if "not checked here: tasty: a suite that names its failures alone (its run fails a listed check no line names)" not in out:
            failures.append(f"a list named at run time alone: {text}")
        if out[-1] != "stale entries: tests/pending/cases.txt tests/board/suites/interp.tsv tests/pending/jvm-assertions.txt":
            failures.append(f"the last line: {out[-1]}")
    for f in failures:
        print("FAIL stale_entries self-test: " + f)
    print("stale_entries self-test: " + ("ok" if not failures else f"{len(failures)} failed"))
    return 1 if failures else 0


def main(argv):
    if argv[:1] == ["--self-test"]:
        return self_test()
    if any(a.startswith("-") for a in argv):
        print(__doc__, file=sys.stderr)
        return 2
    out, stale = check(ROOT, set(argv) or None)
    print("\n".join(out))
    return 1 if stale else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
