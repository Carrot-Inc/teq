#!/usr/bin/env python3
"""table.py <cells dir>: the package-object table from table.sh's legs. Per cell: scalac's output (the oracle) and
whether each leg's output equals it; the paths two products of one teq build share (from teq-products.json); whether
the JVM builds at 1 and 8 workers wrote the same files; the TASTy paths scalac writes that teq does not and the
reverse. table.py --self-test: the judge on planted cells, each a good cell with one record made bad (missing, empty,
cut, past its bound, a wrong value, a cell held twice, a stray or a missing cell), which it must fail with its reason
named, and good cells, which it must pass.

Every exit is taken by a list of the admitted ones: a build's exit 0, or a compiler's own refusal by name, its exit with
its line on its log (the interpreter's in its output): scalac's exit 1 with `-- Error: ..` or `-- [E161] Naming Error:
..` for scalac's builds, teq's exit 1 with `<file>:<line>:<column>: error: ..` or `<n> definitions cannot be written as
TASTy`, its exit 3 with `<n> constructs not supported by the JVM backend` for teq's; a run's exit 0. Every other exit
is not admitted: a signal's (128 and its number, 137 for a kill), 2, the bound passed (124), a refusal's exit without
its line, an exit missing, empty, a NOT JUDGED mark or other text. A leg's builds are read in order up to the first
that refuses, which ends the leg (the split build's user and scalac reading its library are built over the library's
products, which a refused library build does not write), and a run after a refused build is not read.

The cells are cells.py's, each directory's cell.json naming its cell: a directory of no cell cells.py makes (a stray),
a cell it makes twice or one whose name another directory's cell.json gives (a duplicate), and a cell it makes with
no directory (missing) are NOT JUDGED and named. A cell is NOT JUDGED, named with its reason, when a record of it is
missing: scalac's build exit and, where it built, its run's exit and output, where it refused, its log; each leg's
build exits and, where it built, its run's exit (the interpreter's build is its run) and its output; the
teq-products.json of each JVM build that built; when an exit is not admitted; or when a record is not whole:
scalac's output empty or its last line unended (the cells print whole lines), a teq-products.json empty, not whole
JSON or listing no products. Where scalac builds, a leg
differs when one of its builds refuses or its output is not scalac's, its last line ended; where scalac refuses,
every teq build (the JVM's and JavaScript's, at 1 and at 8 workers) must refuse. Exits 1 when a cell is not judged, a
path is shared, the worker counts differ, or a leg differs."""
import collections
import hashlib
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

LEGS = ["jvm1", "jvm8", "js1", "js8", "interp", "split", "scalac-reads"]
# Per leg, the builds it stands on (it is built where each exits 0); its run's exit is `<leg>.run-exit`, but for the
# interpreter, whose build is its run, and its output `<leg>.out`.
BUILDS = {"jvm1": ["jvm1"], "jvm8": ["jvm8"], "js1": ["js1"], "js8": ["js8"], "interp": ["interp"],
          "split": ["split-lib", "split-user"], "scalac-reads": ["split-lib", "scalac-reads"]}
RUN_LEGS = set(LEGS) - {"interp"}
# The teq builds that must refuse a program scalac refuses.
REFUSERS = ("jvm1", "jvm8", "js1", "js8")
TIMEOUT = "124"
ANSI = re.compile(r"\x1b\[[0-9;]*m")
# Per compiler, its refusals by name: per exit, the lines one of which its log holds when it refuses with that exit.
# scalac's exit 1 with its header (`-- [E161] Naming Error: <file>:..`, `-- Error: ..`); teq's exit 1 with its error
# line or the summary of definitions it cannot write as TASTy (after their `<file>: cannot write TASTy for ..` lines),
# and its exit 3 with the summary of constructs its JVM backend does not support (src/main.rs prints each before that
# exit).
REFUSALS = {
    "scalac": {"1": [("error", re.compile(r"^-- (?:\[E\d+\] )?(?:[A-Za-z]+ )*Error: "))]},
    "teq": {"1": [("error", re.compile(r"^.+?:\d+:\d+: error: ")),
                  ("TASTy summary", re.compile(r"^\d+ definitions? cannot be written as TASTy$"))],
            "3": [("JVM backend summary", re.compile(r"^\d+ constructs? not supported by the JVM backend$"))]},
}
# The builds that are scalac's (every other is teq's), and the records that hold a build's errors where its log does
# not (the interpreter writes its errors with its output).
SCALAC_BUILDS = {"scalac", "scalac-reads"}
ERRORS_IN = {"interp": "interp.out"}


def read(p):
    """A record's text; None where there is no such file."""
    try:
        return open(p, encoding="utf-8", errors="replace").read()
    except OSError:
        return None


def code(r, name):
    """A recorded exit code, the file's text stripped; None where there is no record (no file, or an empty one)."""
    t = read(os.path.join(r, name))
    return t.strip() if t is not None and t.strip() != "" else None


def exit_problem(r, name, who, what, build=None):
    """Why the record `name` of `who`'s `what` cannot be judged; None where its exit is admitted: 0, or for the build
    `build` one of its compiler's refusals by name (REFUSALS: the exit with its line on its log). Not admitted: an exit
    missing or empty, a NOT JUDGED mark or other text, the bound passed (124), a signal's (128 and its number), a
    refusal's exit without its line, and every other exit; a run's but 0."""
    ex = code(r, name)
    if ex is None:
        return f"no record of {who}'s {what}"
    if ex.startswith("NOT JUDGED"):
        return f"{who}'s {what} {ex.splitlines()[0]}"
    if not re.fullmatch(r"[0-9]+", ex):
        return f"{who}'s {what} recorded as `{ex.splitlines()[0][:30]}`, not an exit"
    if ex == "0":
        return None
    compiler = "scalac" if build in SCALAC_BUILDS else "teq"
    named = REFUSALS[compiler].get(ex, []) if build is not None else []
    if named:
        log = read(os.path.join(r, ERRORS_IN.get(build, f"{build}.log")))
        if log is None:
            return f"no record of {who}'s {what} log"
        if any(pattern.match(ANSI.sub("", l)) for _, pattern in named for l in log.splitlines()):
            return None
        return f"{who}'s {what} exited {ex} with no {compiler} {' or '.join(n for n, _ in named)} on its log"
    if ex == TIMEOUT:
        return f"{who}'s {what} past its bound"
    return f"{who}'s {what} exited {ex}" + (f" (signal {int(ex) - 128})" if int(ex) > 128 else "")


def scalac_problem(r):
    """Why scalac's record cannot be the oracle; None where it can."""
    if code(r, "scalac.exit") not in (None, "0") and read(os.path.join(r, "scalac.log")) is None:
        return "no record of scalac's log"
    why = exit_problem(r, "scalac.exit", "scalac", "build", "scalac")
    if why:
        return why
    if code(r, "scalac.exit") == "0":
        why = exit_problem(r, "scalac.run-exit", "scalac", "run")
        if why:
            return why
        out = read(os.path.join(r, "scalac.out"))
        if out is None:
            return "no record of scalac's output"
        if not out.strip():
            return "scalac's run printed nothing"
        if not out.endswith("\n"):
            return "scalac's output cut (its last line unended)"
    return None


def leg_problem(r, leg):
    """Why a leg's record cannot be judged; None where it can. Its builds are read up to the first that refuses."""
    who = f"teq's {leg} leg"
    for b in BUILDS[leg]:
        why = exit_problem(r, f"{b}.exit", who, "build" if b == leg else f"{b} build", b)
        if why:
            return why
        if code(r, f"{b}.exit") != "0":
            return None
    if leg in RUN_LEGS:
        why = exit_problem(r, f"{leg}.run-exit", who, "run")
        if why:
            return why
    if read(os.path.join(r, f"{leg}.out")) is None:
        return f"no record of {who}'s output"
    return None


def manifest_problem(d, who):
    """Why a JVM build's teq-products.json cannot be read for its products: missing, empty, not whole JSON, or listing
    no products; None where it can."""
    text = read(os.path.join(d, "teq-products.json"))
    if text is None:
        return f"no record of {who}'s teq-products.json"
    if not text.strip():
        return f"{who}'s teq-products.json empty"
    try:
        m = json.loads(text)
    except ValueError:
        return f"{who}'s teq-products.json not whole JSON (cut)"
    products = m.get("products") if isinstance(m, dict) else None
    if not isinstance(products, list) or not products or not all(isinstance(p, dict) for p in products):
        return f"{who}'s teq-products.json lists no products"
    return None


def files_digest(d):
    h = hashlib.sha256()
    if not os.path.isdir(d):
        return None
    for root, _, fs in sorted(os.walk(d)):
        for f in sorted(fs):
            if f == "teq-products.json":
                continue
            p = os.path.join(root, f)
            h.update(os.path.relpath(p, d).encode())
            h.update(open(p, "rb").read())
    return h.hexdigest()[:16]


def shared(d):
    """The TASTy paths two products of the manifest name (read whole: manifest_problem passed it)."""
    m = json.load(open(os.path.join(d, "teq-products.json"), encoding="utf-8"))
    c = collections.Counter(p["tasty"] for p in m["products"] if p.get("tasty"))
    return sorted(k for k, n in c.items() if n > 1)


def tasty_paths(d):
    out = set()
    for root, _, fs in os.walk(d):
        for f in fs:
            if f.endswith(".tasty"):
                out.add(os.path.relpath(os.path.join(root, f), d))
    return out


def manifest_names():
    """The names of cells.py's cells in its order, a name it makes twice listed twice."""
    sys.dont_write_bytecode = True
    if HERE not in sys.path:
        sys.path.insert(0, HERE)
    import cells
    return [c[0] for c in cells.cells()]


def cell_name(cell):
    """The name the cell's cell.json gives; None where it is missing, no JSON object or names nothing."""
    try:
        spec = json.load(open(os.path.join(cell, "cell.json"), encoding="utf-8"))
    except (OSError, ValueError):
        return None
    return spec.get("name") if isinstance(spec, dict) and isinstance(spec.get("name"), str) else None


def held(root, dirs, manifest):
    """{name: why} for each directory that does not hold one cell of the manifest, held once (a stray, a duplicate, a
    cell.json missing or naming another cell), and each cell of the manifest with no directory."""
    made = collections.Counter(manifest)
    names = {c: cell_name(os.path.join(root, c)) for c in dirs}
    claims = collections.defaultdict(list)
    for c, n in names.items():
        if n is not None and n != c:
            claims[n].append(c)
    why = {}
    for c in dirs:
        n = names[c]
        if made[c] == 0:
            why[c] = "a stray: a directory of no cell cells.py makes"
        elif made[c] > 1:
            why[c] = f"a duplicate: cells.py makes {made[c]} cells of this name"
        elif n is None:
            why[c] = "no record of its cell.json (missing, or naming no cell)"
        elif n != c:
            why[c] = (f"a duplicate: its cell.json names {n}, which {n}'s directory holds" if n in dirs
                      else f"its cell.json names {n!r}, not its own cell")
        elif claims.get(c):
            why[c] = f"a duplicate: {', '.join(claims[c])}'s cell.json names this cell too"
    for c, k in made.items():
        if c not in why and c not in dirs:
            why[c] = "missing: a cell cells.py makes, with no directory" + ("" if k == 1 else f", made {k} times")
    return why


def judge(root, out=sys.stdout, manifest=None):
    """The table of the cells under `root`, which must be the manifest's (cells.py's cells' names where none is
    given); returns its exit."""
    if manifest is None:
        manifest = manifest_names()
    dirs = sorted(os.listdir(root))
    membership = held(root, dirs, manifest)
    names = sorted(set(dirs) | set(manifest))
    bad, judged, unjudged = 0, 0, []
    print("| cell | scalac's output | " + " | ".join(LEGS) + " | shared paths | 1 = 8 workers | TASTy only scalac's | only teq's |", file=out)
    print("|---|---|" + "---|" * len(LEGS) + "---|---|---|---|", file=out)
    for c in names:
        why = membership.get(c)
        r = os.path.join(root, c, "r")
        if why is None:
            why = scalac_problem(r)
        if why is None:
            whys = [w for w in (leg_problem(r, leg) for leg in LEGS) if w]
            if not whys and code(r, "scalac.exit") == "0" and all(code(r, f"{leg}.exit") == "0" for leg in ("jvm1", "jvm8")):
                whys = [w for w in (manifest_problem(os.path.join(r, leg), f"teq's {leg} leg") for leg in ("jvm1", "jvm8")) if w]
            why = "; ".join(whys) or None
        if why:
            unjudged.append((c, why))
            print(f"| {c} | NOT JUDGED: {why} |" + " |" * (len(LEGS) + 4), file=out)
            continue
        judged += 1
        if code(r, "scalac.exit") != "0":
            # scalac refuses the program: so must every teq build of it.
            refused = all(code(r, f"{leg}.exit") != "0" for leg in REFUSERS)
            if not refused:
                bad += 1
            print(f"| {c} | scalac refuses | " + " | ".join("refuses" if refused else "NO: builds" for _ in LEGS) + " | | | | |", file=out)
            continue
        oracle = read(os.path.join(r, "scalac.out"))
        legs = []
        for leg in LEGS:
            refusal = [(b, code(r, f"{b}.exit")) for b in BUILDS[leg] if code(r, f"{b}.exit") != "0"][:1]
            if refusal:
                legs.append(f"NO: refuses ({refusal[0][0]} {refusal[0][1]})")
                continue
            o = read(os.path.join(r, f"{leg}.out"))
            if o.strip() == oracle.strip() and o.endswith("\n"):
                legs.append("=")
            else:
                cut = "" if not o or o.endswith("\n") else ", cut (its last line unended)"
                legs.append(f"NO: `{o.strip().splitlines()[-1][:40] if o.strip() else ''}`{cut}")
        if all(code(r, f"{leg}.exit") == "0" for leg in ("jvm1", "jvm8")):
            sh = shared(os.path.join(r, "jvm1")) + shared(os.path.join(r, "jvm8"))
            same = files_digest(os.path.join(r, "jvm1")) == files_digest(os.path.join(r, "jvm8"))
        else:
            sh, same = [], False
        sc, tq = tasty_paths(os.path.join(r, "scalac")), tasty_paths(os.path.join(r, "jvm1"))
        only_sc, only_tq = sorted(sc - tq), sorted(tq - sc)
        if sh or not same or any(x != "=" for x in legs):
            bad += 1
        print(f"| {c} | `{oracle.strip()}` | " + " | ".join(legs) + f" | {' '.join(sorted(set(sh))) or '0'} | {'yes' if same else 'NO'} | "
              f"{' '.join(only_sc) or '-'} | {' '.join(only_tq) or '-'} |", file=out)
    for c, why in unjudged:
        print(f"NOT JUDGED {c}: {why}", file=out)
    print(f"package-objects: {len(names)} cells: {judged} judged, {len(unjudged)} not judged; {bad} with a shared path, "
          f"a worker difference or an output not scalac's", file=out)
    return 1 if bad or unjudged else 0


def planted(tmp):
    """The self-test's cells, written under `tmp`: per case a dict of its name, its cells directory, the manifest it is
    judged against (None: cells.py's), whether the judge must pass it, and the rows it must print, each with a text
    the row must hold."""
    manifest_text = json.dumps({"products": [{"source": "lib/package.scala", "tasty": "p/package.tasty", "classes": ["p/package.class"]},
                                             {"source": "user/Main.scala", "tasty": "user/run.tasty", "classes": ["user/run.class"]}]})
    good = {"scalac.exit": "0\n", "scalac.run-exit": "0\n", "scalac.out": "(1,2)\n", "scalac.log": "",
            "scalac/p/package.tasty": "T", "scalac/user/run.tasty": "U",
            "interp.exit": "0\n", "interp.out": "(1,2)\n",
            "split-lib.exit": "0\n", "split-lib.log": "", "split-user.exit": "0\n", "split-user.log": "",
            "split.run-exit": "0\n", "split.out": "(1,2)\n",
            "scalac-reads.exit": "0\n", "scalac-reads.log": "", "scalac-reads.run-exit": "0\n", "scalac-reads.out": "(1,2)\n"}
    for leg in REFUSERS:
        good.update({f"{leg}.exit": "0\n", f"{leg}.run-exit": "0\n", f"{leg}.out": "(1,2)\n", f"{leg}.log": ""})
    for leg in ("jvm1", "jvm8"):
        good.update({f"{leg}/p/package.tasty": "T", f"{leg}/p/package.class": "C", f"{leg}/user/run.tasty": "U",
                     f"{leg}/user/run.class": "R", f"{leg}/teq-products.json": manifest_text})
    error = "-- [E161] Naming Error: lib/Lib.scala:6:6 ---\n6 |  def shared(x: Int): Int = x + 1\n  |  ^^^\n1 error found\n"
    teq_error = "lib/Lib.scala:6:7: error: shared is already defined as method shared in lib/Lib.scala\n1 error found\n"
    # scalac and every teq build refuse with their errors; the split build's user and scalac reading its library are
    # built over the library's products, which its refusal did not write.
    refused = dict(good, **{"scalac.exit": "1\n", "scalac.log": error, "interp.exit": "1\n", "interp.out": teq_error,
                            "split-lib.exit": "1\n", "split-lib.log": teq_error,
                            "split-user.exit": "2\n", "split-user.log": "cannot open r/split-lib: No such file or directory (os error 2)\n",
                            "scalac-reads.exit": "1\n", "scalac-reads.log": "error: bad path\n"},
                   **{f"{leg}.exit": "1\n" for leg in REFUSERS}, **{f"{leg}.log": teq_error for leg in REFUSERS})
    no_outputs = {f"{leg}.out": None for leg in LEGS + ["scalac"]}
    empty_outputs = {f"{leg}.out": "" for leg in LEGS + ["scalac"]}
    no_products = {f"{leg}/{f}": None for leg in ("jvm1", "jvm8") for f in ("p/package.tasty", "p/package.class", "user/run.tasty", "user/run.class", "teq-products.json")}
    shared_text = json.dumps({"products": [{"source": "lib/Lib.scala", "tasty": "p/package.tasty", "classes": []},
                                           {"source": "lib/package.scala", "tasty": "p/package.tasty", "classes": ["p/package.class"]}]})

    def cell(root, name, files, named=None, spec=True):
        """Cell `name` under `root`: its cell.json naming `named` (the cell itself by default; none where `spec` is
        false) and its records, a path of `r` each, a record given None left out."""
        d = os.path.join(root, name)
        os.makedirs(os.path.join(d, "r"))
        if spec:
            with open(os.path.join(d, "cell.json"), "w") as f:
                json.dump({"name": named or name}, f)
        for rel, text in files.items():
            if text is None:
                continue
            p = os.path.join(d, "r", rel)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            with open(p, "w", encoding="utf-8") as f:
                f.write(text)

    # (name, records, must pass, a text the row must print)
    one = [
        ("passes", good, True, "| = | = | = | = | = | = | = | 0 | yes |"),
        ("refusal-met", refused, True, "scalac refuses | refuses"),
        # a record missing, of scalac or of teq
        ("no-scalac-exit", dict(refused, **{"scalac.exit": None}), False, "no record of scalac's build"),
        ("no-scalac-run-exit", dict(good, **{"scalac.run-exit": None}), False, "no record of scalac's run"),
        ("no-scalac-output", dict(good, **{"scalac.out": None}), False, "no record of scalac's output"),
        ("no-output-on-either-side", dict(good, **no_outputs), False, "no record of scalac's output"),
        ("no-scalac-log-of-a-refusal", dict(refused, **{"scalac.log": None}), False, "no record of scalac's log"),
        ("no-teq-exit", dict(good, **{"jvm8.exit": None}), False, "no record of teq's jvm8 leg's build"),
        ("no-teq-run-exit", dict(good, **{"js1.run-exit": None}), False, "no record of teq's js1 leg's run"),
        ("no-teq-output", dict(good, **{"split.out": None}), False, "no record of teq's split leg's output"),
        ("no-split-lib-exit", dict(good, **{"split-lib.exit": None}), False, "no record of teq's split leg's split-lib build"),
        ("no-teq-refusal-exit", dict(refused, **{"js8.exit": None}), False, "no record of teq's js8 leg's build"),
        ("no-products-directories", dict(good, **no_products), False, "no record of teq's jvm1 leg's teq-products.json"),
        ("no-products-directory-at-8", dict(good, **{k: v for k, v in no_products.items() if k.startswith("jvm8/")}), False,
         "no record of teq's jvm8 leg's teq-products.json"),
        ("no-manifest", dict(good, **{"jvm8/teq-products.json": None}), False, "no record of teq's jvm8 leg's teq-products.json"),
        ("no-cell-json", good, False, "no record of its cell.json"),
        # an empty record
        ("empty-scalac-exit", dict(good, **{"scalac.exit": ""}), False, "no record of scalac's build"),
        ("empty-scalac-output", dict(good, **{"scalac.out": ""}), False, "scalac's run printed nothing"),
        ("empty-output-on-either-side", dict(good, **empty_outputs), False, "scalac's run printed nothing"),
        ("empty-teq-output", dict(good, **{"interp.out": ""}), False, "NO: ``"),
        ("empty-scalac-log-of-a-refusal", dict(refused, **{"scalac.log": ""}), False, "scalac's build exited 1 with no scalac error on its log"),
        ("empty-manifest", dict(good, **{"jvm8/teq-products.json": ""}), False, "teq's jvm8 leg's teq-products.json empty"),
        ("manifest-of-no-products", dict(good, **{"jvm1/teq-products.json": '{"products": []}'}), False,
         "teq's jvm1 leg's teq-products.json lists no products"),
        # a cut record
        ("cut-scalac-output", dict(good, **{"scalac.out": "(1,2)"}), False, "scalac's output cut"),
        ("cut-teq-output", dict(good, **{"js8.out": "(1,2)"}), False, "NO: `(1,2)`, cut (its last line unended)"),
        ("cut-manifest", dict(good, **{"jvm1/teq-products.json": manifest_text[:40]}), False,
         "teq's jvm1 leg's teq-products.json not whole JSON"),
        ("cut-scalac-log-of-a-refusal", dict(refused, **{"scalac.log": error.split("\n", 1)[1].rsplit("1 error", 1)[0]}), False,
         "scalac's build exited 1 with no scalac error on its log"),
        # past its bound, or marked not judged
        ("scalac-timeout", dict(refused, **{"scalac.exit": "124\n"}), False, "scalac's build past its bound"),
        ("scalac-run-timeout", dict(good, **{"scalac.run-exit": "124\n"}), False, "scalac's run past its bound"),
        ("teq-timeout", dict(good, **{"jvm1.exit": "124\n"}), False, "teq's jvm1 leg's build past its bound"),
        ("teq-run-timeout", dict(good, **{"js8.run-exit": "124\n"}), False, "teq's js8 leg's run past its bound"),
        ("split-user-timeout", dict(good, **{"split-user.exit": "124\n"}), False, "teq's split leg's split-user build past its bound"),
        ("teq-timeout-where-scalac-refuses", dict(refused, **{"jvm8.exit": "124\n"}), False, "teq's jvm8 leg's build past its bound"),
        ("scalac-marked-not-judged", dict(refused, **{"scalac.exit": "NOT JUDGED: its lock not taken\n"}), False, "scalac's build NOT JUDGED"),
        ("teq-marked-not-judged", dict(good, **{"interp.exit": "NOT JUDGED: x\n"}), False, "teq's interp leg's build NOT JUDGED"),
        ("scalac-exit-not-a-number", dict(refused, **{"scalac.exit": "Killed\n"}), False, "recorded as `Killed`, not an exit"),
        ("scalac-run-failed", dict(good, **{"scalac.run-exit": "1\n"}), False, "scalac's run exited 1"),
        # an exit not admitted: a signal's, 2, or 1 with no error of its compiler, on each side
        ("scalac-killed", {"scalac.exit": "137\n", "scalac.log": ""}, False, "scalac's build exited 137 (signal 9)"),
        ("scalac-exit-2", dict(refused, **{"scalac.exit": "2\n"}), False, "scalac's build exited 2"),
        ("scalac-exit-139", dict(refused, **{"scalac.exit": "139\n"}), False, "scalac's build exited 139 (signal 11)"),
        ("scalac-run-killed", dict(good, **{"scalac.run-exit": "137\n"}), False, "scalac's run exited 137 (signal 9)"),
        ("teq-exit-2", dict(good, **{"jvm1.exit": "2\n", "jvm1.log": "cannot open x\n"}), False, "teq's jvm1 leg's build exited 2"),
        ("teq-exit-139", dict(good, **{"js8.exit": "139\n"}), False, "teq's js8 leg's build exited 139 (signal 11)"),
        ("teq-killed-where-scalac-refuses", dict(refused, **{"jvm8.exit": "137\n"}), False, "teq's jvm8 leg's build exited 137 (signal 9)"),
        ("teq-exit-1-without-its-error", dict(refused, **{"js1.log": "something failed\n"}), False, "teq's js1 leg's build exited 1 with no teq error or TASTy summary on its log"),
        ("teq-exit-1-without-its-log", dict(refused, **{"js1.log": None}), False, "no record of teq's js1 leg's build log"),
        ("interp-exit-1-without-its-error", dict(good, **{"interp.exit": "1\n", "interp.out": "Exception in thread main\n"}), False,
         "teq's interp leg's build exited 1 with no teq error or TASTy summary on its log"),
        ("split-user-exit-2-over-its-library", dict(good, **{"split-user.exit": "2\n"}), False, "teq's split leg's split-user build exited 2"),
        ("scalac-reads-exit-2", dict(good, **{"scalac-reads.exit": "2\n"}), False, "teq's scalac-reads leg's build exited 2"),
        ("scalac-reads-exit-1-without-its-error", dict(good, **{"scalac-reads.exit": "1\n", "scalac-reads.log": "error: x\n"}), False,
         "teq's scalac-reads leg's build exited 1 with no scalac error on its log"),
        ("teq-run-crashed", dict(good, **{"jvm8.run-exit": "139\n"}), False, "teq's jvm8 leg's run exited 139 (signal 11)"),
        # teq's refusals by name: each exit with its own line, and without it
        ("refused-writing-tasty-where-scalac-refuses", dict(refused, **{"jvm1.log": "lib/Lib.scala: cannot write TASTy for x\n1 definition cannot be written as TASTy\n"}), True,
         "scalac refuses | refuses"),
        ("refused-by-the-jvm-backend-where-scalac-refuses", dict(refused, **{"jvm8.exit": "3\n", "jvm8.log": "lib/Lib.scala: x\n2 constructs not supported by the JVM backend\n"}), True,
         "scalac refuses | refuses"),
        ("refused-by-the-jvm-backend-where-scalac-builds", dict(good, **{"jvm1.exit": "3\n", "jvm1.log": "1 construct not supported by the JVM backend\n"}), False,
         "NO: refuses (jvm1 3)"),
        ("cannot-write-tasty-without-the-summary", dict(refused, **{"js8.log": "lib/Lib.scala: cannot write TASTy for x\n"}), False,
         "teq's js8 leg's build exited 1 with no teq error or TASTy summary on its log"),
        ("teq-exit-3-without-its-line", dict(refused, **{"jvm1.exit": "3\n", "jvm1.log": "something failed\n"}), False,
         "teq's jvm1 leg's build exited 3 with no teq JVM backend summary on its log"),
        ("teq-exit-2-with-a-refusal-line", dict(good, **{"jvm8.exit": "2\n", "jvm8.log": "error: r/split-lib/teq-products.json: these products are of ABI 0 (written by teq 0.1.10)\n"}), False,
         "teq's jvm8 leg's build exited 2"),
        ("scalac-exit-3", dict(refused, **{"scalac.exit": "3\n", "scalac.log": error + "3 constructs not supported by the JVM backend\n"}), False, "scalac's build exited 3"),
        ("split-run-exit-1", dict(good, **{"split.run-exit": "1\n"}), False, "teq's split leg's run exited 1"),
        # a wrong value
        ("wrong-output", dict(good, **{"jvm8.out": "(1,3)\n"}), False, "NO: `(1,3)`"),
        ("refuses-where-scalac-builds", dict(good, **{"js1.exit": "1\n", "js1.log": teq_error}), False, "NO: refuses (js1 1)"),
        ("split-library-refused-where-scalac-builds", dict(good, **{"split-lib.exit": "1\n", "split-lib.log": teq_error, "split-user.exit": "2\n"}), False,
         "NO: refuses (split-lib 1)"),
        ("builds-where-scalac-refuses", dict(refused, **{"jvm1.exit": "0\n"}), False, "NO: builds"),
        ("shared-path", dict(good, **{"jvm1/teq-products.json": shared_text}), False, "| p/package.tasty | yes |"),
        ("worker-difference", dict(good, **{"jvm8/p/package.class": "D"}), False, "| 0 | NO |"),
    ]
    cases = []
    for name, files, passes, word in one:
        root = os.path.join(tmp, name)
        os.makedirs(root)
        cell(root, name, files, spec=name != "no-cell-json")
        cases.append({"name": name, "root": root, "manifest": [name], "passes": passes, "rows": [(name, word)]})

    # The cells against the manifest: a stray, a duplicate of either kind, a missing cell; each other record good.
    def several(name, cells, manifest, rows):
        root = os.path.join(tmp, name)
        os.makedirs(root)
        for c, kw in cells:
            cell(root, c, good, **kw)
        cases.append({"name": name, "root": root, "manifest": manifest, "passes": False, "rows": rows})

    several("stray", [("c", {}), ("s", {})], ["c"], [("s", "a stray"), ("c", "| = |")])
    several("duplicate-directory", [("c", {}), ("d", {"named": "c"})], ["c", "d"],
            [("d", "a duplicate: its cell.json names c"), ("c", "a duplicate: d's cell.json names this cell too")])
    several("duplicate-in-the-manifest", [("c", {})], ["c", "c"], [("c", "a duplicate: cells.py makes 2 cells")])
    several("missing", [("c", {})], ["c", "gone"], [("gone", "missing: a cell cells.py makes"), ("c", "| = |")])
    # cells.py's own manifest: one of its cells here, its others missing, and a stray.
    several("cells-py-manifest", [("braces--one--def", {}), ("not-a-cell", {})], None,
            [("braces--one--def", "| = |"), ("braces--two--def", "missing"), ("not-a-cell", "a stray")])
    return cases


def self_test():
    """The planted cells through the judge: each as expected, its rows printed with their texts."""
    import io
    import shutil
    import tempfile

    tmp = tempfile.mkdtemp(prefix="package-objects-self-test-")
    bad, cases = [], []
    try:
        cases = planted(tmp)
        for case in cases:
            buf = io.StringIO()
            try:
                rc = judge(case["root"], out=buf, manifest=case["manifest"])
            except Exception as e:
                bad.append(f"{case['name']}: the judge stopped: {type(e).__name__}: {e}")
                continue
            lines = buf.getvalue().splitlines()
            missed = []
            for row, word in case["rows"]:
                got = [l for l in lines if l.startswith(f"| {row} |")]
                if not got or word not in got[0]:
                    missed.append(got[0] if got else f"no row {row}")
            if (rc == 0) != case["passes"] or missed:
                bad.append(f"{case['name']}: exit {rc}, {'passes' if case['passes'] else 'fails'} expected; {'; '.join(missed) or 'rows as expected'}")
    finally:
        if tmp.startswith(tempfile.gettempdir()):
            shutil.rmtree(tmp, ignore_errors=True)
    for b in bad:
        print(f"package-objects self-test: NOT AS EXPECTED: {b}")
    print(f"package-objects self-test: {len(cases)} cases, {len(cases) - len(bad)} as expected")
    return 1 if bad else 0


def main():
    if sys.argv[1:] == ["--self-test"]:
        return self_test()
    return judge(sys.argv[1])


if __name__ == "__main__":
    sys.exit(main())
