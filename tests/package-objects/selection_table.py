#!/usr/bin/env python3
"""selection_table.py <cells dir> <teq version> <teq products number>: per selection cell, scalac's answer and each of
teq's legs against it. selection_table.py --self-test: the judge on planted cells, each a good cell with one record
made bad (missing, empty, cut, past its bound, a wrong value, a cell held twice, a stray or a missing cell), which it
must fail with its reason named, and good cells, which it must pass.

An answer is the client's build exit, its run's output (a refusal: no output), the text of the dropStale warning or
error (the lines from `Toplevel definition` through `Keeping only the definition in ..` or `.. removed from the
classpath.`) and, for a refusal, the places of its errors (each error's file and line); colour codes taken out and the
cell's own directory written `<cell>`. A leg is the same as scalac's when its answer is, its output's last line ended
as scalac's is. A refusal's error texts other than dropStale's are not compared, and nor are its columns, which the
table prints (scalac's read as teq prints columns, from 1): teq places an error of a selection at the selection's start
where scalac places it at its name.

Every exit is taken by a list of the admitted ones: a build's exit 0, or a compiler's own refusal by name, its exit with
its line on its log: scalac's exit 1 with `-- Error: ..` or `-- [E161] Naming Error: ..`, Scala 2's with `<file>:<line>:
error: ..`, teq's exit 1 with `<file>:<line>:<column>: error: ..` or `<n> definitions cannot be written as TASTy`, its
exit 3 with `<n> constructs not supported by the JVM backend`, and in a migration cell a teq leg's exit 2 with the
filled REFUSAL line (teq's refusal of its class path); a run's exit 0. Every other exit is not admitted: a signal's
(128 and its number, 137 for a kill), the bound passed (124), a refusal's exit without its line, every other exit, an
exit missing, empty, a NOT JUDGED mark or other text. A run after a refused build is not read.

The cells are selection.py's, each directory's cell.json naming its cell: a directory of no cell selection.py makes (a
stray), a cell it makes twice or one whose name another directory's cell.json gives (a duplicate), and a cell it makes
with no directory (missing) are NOT JUDGED and named. A cell is NOT JUDGED, named with its reason, when a record of it
is missing: its cell.json, an upstream module's build exit (of each module cell.json names), scalac's or a leg's
build exit and log and, where it built, its run's exit and output; when an exit is not admitted; when scalac's client
built but did not run; or when scalac's record is not whole: its output's last line unended (the cells print whole
lines), a refusal naming no error place, a dropStale message without its closing line. A migration cell
(`migration--*`: upstream products an earlier teq wrote, of number 0) is judged against teq's refusal of products of
another number: each leg refused, with the line of src/products.rs's REFUSAL, filled for the first upstream directory
of the class path and checked whole, which names the directory to rebuild (scalac reads them; its answer is shown, and
its record is required as any cell's). A cell whose upstream module does not build (its writer refuses it: exit 1
with its errors) has nothing to judge and is named so. The summary counts the cells judged, not judged and with nothing to
judge, and the legs whose answer differs; exits 1 when a cell is not judged or a leg differs."""
import collections
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

LEGS = ["jvm1", "jvm8", "js", "interp"]
# The legs whose run follows the build as a command of its own (the interpreter's build is its run).
RUN_LEGS = {"jvm1", "jvm8", "js"}
ANSI = re.compile(r"\x1b\[[0-9;]*m")
TIMEOUT = "124"
# scalac's error header (`-- [E161] Naming Error: <file>:<line>:<column> ---`, its column from 0) and teq's error line
# (`<file>:<line>:<column>: error: ..`, its column from 1).
SCALAC_ERROR = re.compile(r"^-- (?:\[E\d+\] )?(?:[A-Za-z]+ )*Error: (.+?):(\d+):(\d+)(?: |$)")
TEQ_ERROR = re.compile(r"^(.+?):(\d+):(\d+): error: ")
# Per compiler, its refusals by name: per exit, the lines one of which its log holds when it refuses with that exit.
# scalac's exit 1 with its header, Scala 2's with `<file>:<line>: error: ..`; teq's exit 1 with its error line or the
# summary of definitions it cannot write as TASTy (after their `<file>: cannot write TASTy for ..` lines), and its exit 3
# with the summary of constructs its JVM backend does not support (src/main.rs prints each before that exit). An
# upstream module's writer is scalac, Scala 2 (`scalac2`) or a teq (`teq`, `teq-<name>`).
REFUSALS = {
    "scalac": {"1": [("error", re.compile(r"^-- (?:\[E\d+\] )?(?:[A-Za-z]+ )*Error: "))]},
    "scalac2": {"1": [("error", re.compile(r"^.+?:\d+: error: "))]},
    "teq": {"1": [("error", re.compile(r"^.+?:\d+:\d+: error: ")),
                  ("TASTy summary", re.compile(r"^\d+ definitions? cannot be written as TASTy$"))],
            "3": [("JVM backend summary", re.compile(r"^\d+ constructs? not supported by the JVM backend$"))]},
}
# teq's exit where it refuses its class path (src/main.rs, open_jars), a migration cell's answer with the REFUSAL line.
CLASSPATH_REFUSAL = "2"


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


def exit_problem(r, name, who, what, compiler=None, log=None, also=None):
    """Why the record `name` of `who`'s `what` (a build or a run) cannot be judged; None where its exit is admitted: 0,
    one of `compiler`'s refusals by name (REFUSALS: the exit with its line on the log `log`), or `also`, an exit and the
    line that must be on the log with it. Not admitted: an exit missing or empty, a NOT JUDGED mark or other text, the
    bound passed (124), a signal's (128 and its number), a refusal's exit without its line, and every other exit."""
    ex = code(r, name)
    if ex is None:
        return f"no record of {who}'s {what}"
    if ex.startswith("NOT JUDGED"):
        return f"{who}'s {what} {ex.splitlines()[0]}"
    if not re.fullmatch(r"[0-9]+", ex):
        return f"{who}'s {what} recorded as `{ex.splitlines()[0][:30]}`, not an exit"
    if ex == "0":
        return None
    lines = [ANSI.sub("", l) for l in (read(os.path.join(r, log)) or "").splitlines()] if log else []
    if also is not None and ex == also[0] and also[1] in lines:
        return None
    named = REFUSALS.get(compiler, {}).get(ex, [])
    if named:
        if any(pattern.match(l) for _, pattern in named for l in lines):
            return None
        return f"{who}'s {what} exited {ex} with no {compiler} {' or '.join(n for n, _ in named)} on its log"
    if ex == TIMEOUT:
        return f"{who}'s {what} past its bound"
    return f"{who}'s {what} exited {ex}" + (f" (signal {int(ex) - 128})" if int(ex) > 128 else "")


def selection_text(log, cell):
    """The dropStale message's lines, normalized; '' where there is none."""
    if not log:
        return ""
    lines = [ANSI.sub("", l).replace(cell, "<cell>").strip() for l in log.splitlines()]
    out, on = [], False
    for l in lines:
        l = re.sub(r"^\|\s*", "", l)
        if "Toplevel definition" in l:
            on = True
            out = [l[l.index("Toplevel definition"):]]
            continue
        if on:
            if not l or l.startswith("--") or re.match(r"^\d+ (warning|error)s? found", l):
                break
            out.append(l)
            if l.startswith("Keeping only") or "removed from the classpath" in l:
                break
    return "\n".join(out)


def message_whole(msg):
    """Whether a dropStale message reaches its closing line (`Keeping only the definition in ..` or `One of these files
    should be removed from the classpath.`); no message is whole."""
    if not msg:
        return True
    last = msg.splitlines()[-1]
    return last.startswith("Keeping only") or "removed from the classpath" in last


def error_places(log, cell, scalac):
    """The places of a build's errors, sorted: (file, line, column from 1). Compared by file and line."""
    out = []
    for l in (log or "").splitlines():
        l = ANSI.sub("", l).replace(cell, "<cell>").rstrip()
        m = (SCALAC_ERROR if scalac else TEQ_ERROR).match(l)
        if m:
            out.append((m.group(1), int(m.group(2)), int(m.group(3)) + (1 if scalac else 0)))
    return sorted(out)


def answer(r, leg, cell):
    """(build exit, run output stripped, dropStale message, a refusal's error places, whether the output's last line is
    ended: a refusal's, or an empty output's, is)."""
    ex = code(r, f"{leg}.exit") or ""
    raw = read(os.path.join(r, f"{leg}.out")) or ""
    log = read(os.path.join(r, f"{leg}.log"))
    refused = ex != "0"
    return (ex, (raw.strip() if not refused else ""), selection_text(log, cell),
            (error_places(log, cell, leg == "scalac") if refused else []), refused or raw == "" or raw.endswith("\n"))


def not_judged(r, leg, also=None):
    """Why a leg's (or scalac's) record cannot be judged; None where it can. Its record: its build's exit, admitted, and
    log and, where it built, its run's exit, 0 (the interpreter's build is its run), and its output. `also`: an exit
    and its line admitted besides (a migration cell's refusal)."""
    who = "scalac" if leg == "scalac" else f"teq's {leg} leg"
    if code(r, f"{leg}.exit") is not None and read(os.path.join(r, f"{leg}.log")) is None:
        return f"no record of {who}'s log"
    why = exit_problem(r, f"{leg}.exit", who, "build", "scalac" if leg == "scalac" else "teq", f"{leg}.log", also)
    if why:
        return why
    if code(r, f"{leg}.exit") == "0":
        if leg == "scalac" or leg in RUN_LEGS:
            why = exit_problem(r, f"{leg}.run-exit", who, "run")
            if why:
                return why
        if read(os.path.join(r, f"{leg}.out")) is None:
            return f"no record of {who}'s output"
    return None


def oracle_problem(sx, so, sm, sp, swhole):
    """Why scalac's answer, its records all there, is no oracle: a client that did not run, an output, a refusal or a
    dropStale message cut; None where it is one."""
    if sx == "0" and (not so or "ClassNotFoundException" in so or "Could not find or load main class" in so):
        return "scalac's client built and did not run"
    if sx == "0" and not swhole:
        return "scalac's output cut (its last line unended)"
    if sx != "0" and not sp:
        return "scalac's refusal names no error place (its log empty or cut)"
    if not message_whole(sm):
        return "scalac's dropStale message cut (no closing line)"
    return None


def refusal_template():
    text = read(os.path.join(HERE, "..", "..", "src", "products.rs")) or ""
    m = re.search(r'^pub const REFUSAL: &str = "(.*)";$', text, re.M)
    return m.group(1) if m else None


def refusal(template, up, version, ours):
    """REFUSAL filled for the upstream directory `up` as the client's class path names it."""
    manifest = os.path.join(up, "teq-products.json")
    try:
        m = json.load(open(manifest))
    except (OSError, ValueError):
        return None
    writer = f"teq {m['teq']}" if isinstance(m.get("teq"), str) else "teq"
    filled = {"manifest": manifest, "abi": str(m.get("abi", 0)), "writer": writer, "version": version, "ours": ours, "dir": up}
    for k, v in filled.items():
        template = template.replace("{" + k + "}", v)
    return template


def refused_with(r, leg, line):
    ex = code(r, f"{leg}.exit") or ""
    log = read(os.path.join(r, f"{leg}.log")) or ""
    return ex not in ("", "0") and line in [ANSI.sub("", l) for l in log.splitlines()]


def manifest_names():
    """The names of selection.py's cells in its order, a name it makes twice listed twice."""
    sys.dont_write_bytecode = True
    if HERE not in sys.path:
        sys.path.insert(0, HERE)
    import selection
    return [c[0] for c in selection.cells()]


def cell_spec(cell):
    """The cell's cell.json; None where it is missing or no JSON object."""
    try:
        spec = json.load(open(os.path.join(cell, "cell.json"), encoding="utf-8"))
    except (OSError, ValueError):
        return None
    return spec if isinstance(spec, dict) else None


def held(root, dirs, manifest):
    """{name: why} for each directory that does not hold one cell of the manifest, held once (a stray, a duplicate, a
    cell.json missing or naming another cell), and each cell of the manifest with no directory."""
    made = collections.Counter(manifest)
    specs = {c: cell_spec(os.path.join(root, c)) for c in dirs}
    names = {c: (s.get("name") if s else None) for c, s in specs.items()}
    claims = collections.defaultdict(list)
    for c, n in names.items():
        if isinstance(n, str) and n != c:
            claims[n].append(c)
    why = {}
    for c in dirs:
        n = names[c]
        if made[c] == 0:
            why[c] = "a stray: a directory of no cell selection.py makes"
        elif made[c] > 1:
            why[c] = f"a duplicate: selection.py makes {made[c]} cells of this name"
        elif specs[c] is None:
            why[c] = "no record of its cell.json (missing, or no JSON object)"
        elif n != c:
            why[c] = (f"a duplicate: its cell.json names {n}, which {n}'s directory holds" if n in dirs
                      else f"its cell.json names {n!r}, not its own cell")
        elif claims.get(c):
            why[c] = f"a duplicate: {', '.join(claims[c])}'s cell.json names this cell too"
    for c, k in made.items():
        if c not in why and c not in dirs:
            why[c] = "missing: a cell selection.py makes, with no directory" + ("" if k == 1 else f", made {k} times")
    return why


def judge(root, version, ours, out=sys.stdout, manifest=None):
    """The table of the cells under `root`, which must be the manifest's (selection.py's cells' names where none is
    given); returns (exit, counts)."""
    if manifest is None:
        manifest = manifest_names()
    template = refusal_template()
    dirs = sorted(os.listdir(root))
    membership = held(root, dirs, manifest)
    cells = sorted(set(dirs) | set(manifest))
    differing, judged, nothing = 0, 0, 0
    unjudged = []
    blank = " | ".join("" for _ in LEGS)
    print("| cell | scalac: exit, output, selection message, refusal's places | " + " | ".join(LEGS) + " |", file=out)
    print("|---|---|" + "---|" * len(LEGS), file=out)
    for c in cells:
        if c in membership:
            unjudged.append((c, membership[c]))
            print(f"| {c} | NOT JUDGED: {membership[c]} | {blank} |", file=out)
            continue
        cell = os.path.join(root, c)
        r = os.path.join(cell, "r")
        spec = cell_spec(cell)
        modules = spec.get("modules")
        if not isinstance(modules, list) or not all(isinstance(m, dict) and isinstance(m.get("name"), str) for m in modules):
            unjudged.append((c, "its cell.json names no modules"))
            print(f"| {c} | NOT JUDGED: its cell.json names no modules | {blank} |", file=out)
            continue
        writers = {m["name"]: m.get("writer") for m in modules if m.get("writer") != "client"}
        ups = list(writers)
        problems = [p for p in (exit_problem(r, f"up-{m}.exit", f"upstream module {m}", "build",
                                             writers[m] if writers[m] in ("scalac", "scalac2") else "teq", f"up-{m}.log")
                                for m in ups) if p]
        if problems:
            unjudged.append((c, "; ".join(problems)))
            print(f"| {c} | NOT JUDGED: {'; '.join(problems)} | {blank} |", file=out)
            continue
        failed = [m for m in ups if code(r, f"up-{m}.exit") != "0"]
        if failed:
            nothing += 1
            print(f"| {c} | upstream module {', '.join(failed)} does not build (its writer refuses) | {blank} |", file=out)
            continue
        why = not_judged(r, "scalac")
        sx, so, sm, sp, swhole = answer(r, "scalac", cell)
        if why is None:
            why = oracle_problem(sx, so, sm, sp, swhole)
        line = None
        if c.startswith("migration--"):
            order = spec.get("order") or []
            line = (refusal(template, os.path.join(cell, "r", f"up-{order[0]}"), version, ours)
                    if template and version and order and isinstance(order[0], str) else None)
        if why is None:
            also = (CLASSPATH_REFUSAL, line) if line is not None else None
            legs_why = [(leg, not_judged(r, leg, also)) for leg in LEGS]
            legs_why = [(leg, w) for leg, w in legs_why if w]
            if legs_why:
                why = "; ".join(w for _, w in legs_why)
        if why:
            unjudged.append((c, why))
            print(f"| {c} | NOT JUDGED: {why} | {blank} |", file=out)
            continue
        judged += 1
        if c.startswith("migration--"):
            marks = []
            for leg in LEGS:
                same = line is not None and refused_with(r, leg, line)
                marks.append("refused, the line" if same else "NO: not refused with the ABI line")
                differing += 0 if same else 1
            print(f"| {c} | teq refuses (products of another number; scalac: {sx}, `{so.splitlines()[-1][:30] if so else ''}`) | " + " | ".join(marks) + " |", file=out)
            continue
        marks = []
        for leg in LEGS:
            tx, to, tm, tp, twhole = answer(r, leg, cell)
            if sx != "0":
                same = tx != "0" and tm == sm and [p[:2] for p in tp] == [p[:2] for p in sp]
            else:
                same = (tx, to, tm) == (sx, so, sm) and twhole
            if same:
                marks.append("=")
            else:
                differing += 1
                place = "" if sx == "0" or [p[:2] for p in tp] == [p[:2] for p in sp] else f", at {' '.join(f'{os.path.basename(f)}:{l}:{k}' for f, l, k in tp) or 'no place'}"
                cut = "" if twhole else ", cut (its last line unended)"
                marks.append(f"NO: {tx}, `{to.splitlines()[-1][:30] if to else ''}`, {'msg' if tm else '-'}{place}{cut}")
        msg = (sm.splitlines()[-1][:50] if sm else "-")
        places = "" if sx == "0" else ", at " + (" ".join(f"{os.path.basename(f)}:{l}:{k}" for f, l, k in sp) or "no place")
        print(f"| {c} | {sx}, `{so.splitlines()[-1][:30] if so else ''}`, {msg}{places} | " + " | ".join(marks) + " |", file=out)
    for c, why in unjudged:
        print(f"NOT JUDGED {c}: {why}", file=out)
    print(f"selection: {len(cells)} cells: {judged} judged, {len(unjudged)} not judged, {nothing} with nothing to judge "
          f"(an upstream module its writer refuses); {differing} legs whose answer is not scalac's", file=out)
    return (1 if differing or unjudged else 0), {"judged": judged, "unjudged": len(unjudged), "nothing": nothing, "differing": differing}


def planted(tmp):
    """The self-test's cells, written under `tmp`: per case a dict of its name, its cells directory, the manifest it is
    judged against (None: selection.py's), whether the judge must pass it, and the rows it must print, each with a
    text the row must hold."""
    def put(d, name, text):
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, name), "w", encoding="utf-8") as f:
            f.write(text)

    def cell(root, name, scalac, legs, ups=(), named=None, spec=True):
        """Cell `name` under `root`: its cell.json naming `named` (the cell itself by default; none where `spec` is
        false) and its modules, the upstream ones `ups`, each (name, build exit or None for no record, writer (`teq`
        where left out), log or None); `scalac` and each leg a dict of the record's files (`exit`, `run-exit`, `out`,
        `log`), a file left out a record missing. A record's `<cell>` is the cell's directory and `<refusal>` the
        REFUSAL line filled for its first upstream directory, written by teq 0.1.10 (number 0) and read by teq 9.9.9
        (number 1), whose manifest a writer `teq-old` leaves there."""
        d = os.path.join(root, name)
        r = os.path.join(d, "r")
        os.makedirs(r)
        ups = [tuple(u) + (None,) * (4 - len(u)) for u in ups]
        if spec:
            modules = [{"name": m, "files": ["A.scala"], "writer": w or "teq"} for m, _, w, _ in ups]
            modules.append({"name": "client", "files": ["Client.scala"], "writer": "client"})
            put(d, "cell.json", json.dumps({"name": named or name, "modules": modules, "order": [m for m, *_ in ups], "main": "Client"}))
        for m, ex, w, log in ups:
            if ex is not None:
                put(r, f"up-{m}.exit", ex)
            if log is not None:
                put(r, f"up-{m}.log", log.replace("<cell>", d))
            if w == "teq-old":
                put(os.path.join(r, f"up-{m}"), "teq-products.json", json.dumps({"teq": "0.1.10", "products": []}))
        line = ""
        if ups:
            up = os.path.join(r, f"up-{ups[0][0]}")
            filled = {"manifest": os.path.join(up, "teq-products.json"), "abi": "0", "writer": "teq 0.1.10", "version": "9.9.9", "ours": "1", "dir": up}
            line = refusal_template() or ""
            for k, v in filled.items():
                line = line.replace("{" + k + "}", v)
        for who, files in [("scalac", scalac)] + [(leg, legs.get(leg, legs.get("*", {}))) for leg in LEGS]:
            for k, v in files.items():
                put(r, f"{who}.{k}", v.replace("<cell>", d).replace("<refusal>", line))

    def without(files, key):
        return {k: v for k, v in files.items() if k != key}

    ran = {"exit": "0\n", "run-exit": "0\n", "out": "11\n", "log": ""}
    refused_b = {"exit": "1\n", "log": "-- [E161] Naming Error: <cell>/m/B.scala:2:4 ---\n2 |def value: Int = 22\n"}
    teq_refused_b = {"exit": "1\n", "log": "<cell>/m/B.scala:2:5: error: value is already defined\n"}
    teq_refused_a = {"exit": "1\n", "log": "<cell>/m/A.scala:2:5: error: value is already defined\n"}
    teq_refused_b3 = {"exit": "1\n", "log": "<cell>/m/B.scala:3:5: error: value is already defined\n"}
    teq_refused_b_col = {"exit": "1\n", "log": "<cell>/m/B.scala:2:1: error: value is already defined\n"}
    head = "warning: Toplevel definition value is defined in\n  <cell>/r/up-b/B.class\nand also in\n  <cell>/r/up-a/A.class\n"
    warns = {"exit": "0\n", "run-exit": "0\n", "out": "22\n", "log": head + "Keeping only the definition in <cell>/r/up-b/B.class\n"}
    warns_cut = dict(warns, log=head)
    ups = [("a", "0\n"), ("b", "0\n")]
    migration = {"ups": [("a", "0\n", "teq-old")]}
    abi_refused = {"exit": "2\n", "log": "<refusal>\n"}
    # (name, scalac, legs, keywords of cell(), must pass, a text the row must print)
    one = [
        ("passes", ran, {"*": ran}, {}, True, "="),
        ("refusal-at-the-place", refused_b, {"*": teq_refused_b}, {}, True, "="),
        ("warns-as-scalac", warns, {"*": warns}, {"ups": ups}, True, "="),
        ("upstream-refused", ran, {"*": ran}, {"ups": [("a", "1\n", "teq", "<cell>/a/A.scala:2:5: error: value is already defined\n"), ("b", "0\n")]}, True, "does not build"),
        ("upstream-refused-by-scalac", ran, {"*": ran}, {"ups": [("a", "1\n", "scalac", "-- [E161] Naming Error: <cell>/a/A.scala:2:4 ---\n"), ("b", "0\n", "scalac")]}, True, "does not build"),
        ("upstream-refused-by-scala-2", ran, {"*": ran}, {"ups": [("a", "1\n", "scalac2", "<cell>/a/A.scala:2: error: not found: value given\n"), ("b", "0\n", "scalac")]}, True, "does not build"),
        ("migration--refused-with-the-line", ran, {"*": abi_refused}, migration, True, "| refused, the line | refused, the line | refused, the line | refused, the line |"),
        ("refusal-at-another-column", refused_b, {"*": teq_refused_b_col}, {}, True, "="),
        ("upstream-refused-writing-tasty", ran, {"*": ran}, {"ups": [("a", "1\n", "teq", "<cell>/a/A.scala: cannot write TASTy for x\n1 definition cannot be written as TASTy\n"), ("b", "0\n")]}, True, "does not build"),
        ("upstream-refused-by-the-jvm-backend", ran, {"*": ran}, {"ups": [("a", "3\n", "teq", "<cell>/a/A.scala: x\n2 constructs not supported by the JVM backend\n"), ("b", "0\n")]}, True, "does not build"),
        # a record missing, of scalac or of teq
        ("no-scalac-record", without(refused_b, "exit"), {"*": teq_refused_b}, {}, False, "no record of scalac's build"),
        ("no-scalac-run-record", without(ran, "run-exit"), {"*": ran}, {}, False, "no record of scalac's run"),
        ("no-scalac-output", without(ran, "out"), {"*": ran}, {}, False, "no record of scalac's output"),
        ("no-scalac-log", without(ran, "log"), {"*": ran}, {}, False, "no record of scalac's log"),
        ("no-teq-record", ran, {"*": ran, "interp": without(ran, "exit")}, {}, False, "no record of teq's interp leg's build"),
        ("no-teq-run-record", ran, {"*": ran, "jvm1": without(ran, "run-exit")}, {}, False, "no record of teq's jvm1 leg's run"),
        ("no-teq-output", ran, {"*": ran, "js": without(ran, "out")}, {}, False, "no record of teq's js leg's output"),
        ("no-teq-log", ran, {"*": ran, "jvm8": without(ran, "log")}, {}, False, "no record of teq's jvm8 leg's log"),
        ("no-upstream-record", ran, {"*": ran}, {"ups": [("a", "0\n"), ("b", None)]}, False, "no record of upstream module b's build"),
        ("no-cell-json", ran, {"*": ran}, {"spec": False}, False, "no record of its cell.json"),
        ("migration--no-cell-json", ran, {"*": ran}, {"spec": False}, False, "no record of its cell.json"),
        # an empty record
        ("empty-upstream-record", ran, {"*": ran}, {"ups": [("a", "0\n"), ("b", "")]}, False, "no record of upstream module b's build"),
        ("empty-scalac-output", dict(ran, out=""), {"*": ran}, {}, False, "scalac's client built and did not run"),
        ("empty-teq-output", ran, {"*": ran, "interp": dict(ran, out="")}, {}, False, "NO: 0, ``"),
        ("empty-scalac-log-of-a-refusal", dict(refused_b, log=""), {"*": dict(teq_refused_b, log="")}, {}, False, "scalac's build exited 1 with no scalac error on its log"),
        ("empty-teq-log-of-a-refusal", refused_b, {"*": teq_refused_b, "jvm1": dict(teq_refused_b, log="")}, {}, False, "teq's jvm1 leg's build exited 1 with no teq error or TASTy summary on its log"),
        ("empty-teq-log-of-a-warning", warns, {"*": warns, "js": dict(warns, log="")}, {"ups": ups}, False, "NO: 0, `22`, -"),
        # a cut record
        ("cut-scalac-output", dict(ran, out="11"), {"*": ran}, {}, False, "scalac's output cut"),
        ("cut-teq-output", ran, {"*": ran, "js": dict(ran, out="11")}, {}, False, "cut (its last line unended)"),
        ("cut-scalac-log-of-a-refusal", dict(refused_b, log="2 |def value: Int = 22\n"), {"*": teq_refused_b}, {}, False, "scalac's build exited 1 with no scalac error on its log"),
        ("refusal-naming-no-place", dict(refused_b, log="-- Error: ---\nsomething\n"), {"*": teq_refused_b}, {}, False, "scalac's refusal names no error place"),
        ("cut-scalac-message", warns_cut, {"*": warns}, {"ups": ups}, False, "scalac's dropStale message cut"),
        ("cut-message-on-both-sides", warns_cut, {"*": warns_cut}, {"ups": ups}, False, "scalac's dropStale message cut"),
        ("cut-teq-message", warns, {"*": warns, "jvm8": warns_cut}, {"ups": ups}, False, "NO: 0, `22`, msg"),
        # past its bound, or marked not judged
        ("scalac-timeout", {"exit": "124\n", "log": ""}, {"*": teq_refused_b}, {}, False, "scalac's build past its bound"),
        ("scalac-run-timeout", dict(ran, **{"run-exit": "124\n", "out": ""}), {"*": ran}, {}, False, "scalac's run past its bound"),
        ("teq-timeout", refused_b, {"*": teq_refused_b, "jvm8": {"exit": "124\n", "log": ""}}, {}, False, "teq's jvm8 leg's build past its bound"),
        ("teq-run-timeout", ran, {"*": ran, "js": dict(ran, **{"run-exit": "124\n", "out": ""})}, {}, False, "teq's js leg's run past its bound"),
        ("upstream-timeout", ran, {"*": ran}, {"ups": [("a", "124\n"), ("b", "0\n")]}, False, "upstream module a's build past its bound"),
        ("upstream-marked-not-judged", ran, {"*": ran}, {"ups": [("a", "NOT JUDGED: no writer teq-old\n"), ("b", "0\n")]}, False, "upstream module a's build NOT JUDGED: no writer"),
        ("scalac-marked-not-judged", dict(refused_b, exit="NOT JUDGED: its lock not taken\n"), {"*": teq_refused_b}, {}, False, "scalac's build NOT JUDGED"),
        ("teq-marked-not-judged", ran, {"*": ran, "jvm8": dict(ran, exit="NOT JUDGED: x\n")}, {}, False, "teq's jvm8 leg's build NOT JUDGED"),
        ("scalac-exit-not-a-number", dict(refused_b, exit="Killed\n"), {"*": teq_refused_b}, {}, False, "recorded as `Killed`, not an exit"),
        # an exit not admitted: a signal's, 2, or 1 with no error of its compiler, on each side
        ("upstream-writer-killed", {}, {}, {"ups": [("a", "137\n", "scalac", ""), ("b", "0\n", "scalac")]}, False, "upstream module a's build exited 137 (signal 9)"),
        ("upstream-exit-2", ran, {"*": ran}, {"ups": [("a", "2\n", "teq", "cannot open x\n"), ("b", "0\n")]}, False, "upstream module a's build exited 2"),
        ("upstream-exit-139", ran, {"*": ran}, {"ups": [("a", "0\n"), ("b", "139\n", "scalac", "")]}, False, "upstream module b's build exited 139 (signal 11)"),
        ("upstream-exit-1-without-its-error", ran, {"*": ran}, {"ups": [("a", "1\n", "teq", "something failed\n"), ("b", "0\n")]}, False, "upstream module a's build exited 1 with no teq error"),
        ("upstream-scala-2-exit-1-with-a-teq-line", ran, {"*": ran}, {"ups": [("a", "1\n", "scalac2", "error: <cell>/x/teq-products.json: of ABI 0\n"), ("b", "0\n", "scalac")]}, False, "upstream module a's build exited 1 with no scalac2 error"),
        ("scalac-killed", {"exit": "137\n", "log": ""}, {}, {}, False, "scalac's build exited 137 (signal 9)"),
        ("scalac-exit-2", dict(refused_b, exit="2\n"), {"*": teq_refused_b}, {}, False, "scalac's build exited 2"),
        ("scalac-exit-139", dict(refused_b, exit="139\n"), {"*": teq_refused_b}, {}, False, "scalac's build exited 139 (signal 11)"),
        ("scalac-run-killed", dict(ran, **{"run-exit": "137\n"}), {"*": ran}, {}, False, "scalac's run exited 137 (signal 9)"),
        ("teq-exit-2", ran, {"*": ran, "jvm1": dict(ran, exit="2\n", log="cannot open x\n")}, {}, False, "teq's jvm1 leg's build exited 2"),
        ("teq-exit-139", ran, {"*": ran, "interp": dict(ran, exit="139\n")}, {}, False, "teq's interp leg's build exited 139 (signal 11)"),
        ("teq-killed", refused_b, {"*": teq_refused_b, "js": {"exit": "137\n", "log": ""}}, {}, False, "teq's js leg's build exited 137 (signal 9)"),
        ("teq-run-crashed", ran, {"*": ran, "jvm8": dict(ran, **{"run-exit": "139\n"})}, {}, False, "teq's jvm8 leg's run exited 139 (signal 11)"),
        ("teq-run-exit-1", ran, {"*": ran, "js": dict(ran, **{"run-exit": "1\n"})}, {}, False, "teq's js leg's run exited 1"),
        ("migration--exit-2-without-the-line", ran, {"*": abi_refused, "jvm8": {"exit": "2\n", "log": "cannot open x\n"}}, migration, False, "teq's jvm8 leg's build exited 2"),
        ("upstream-cannot-write-tasty-without-the-summary", ran, {"*": ran}, {"ups": [("a", "1\n", "teq", "<cell>/a/A.scala: cannot write TASTy for x\n"), ("b", "0\n")]}, False,
         "upstream module a's build exited 1 with no teq error or TASTy summary on its log"),
        ("upstream-exit-3-without-its-line", ran, {"*": ran}, {"ups": [("a", "3\n", "teq", "something failed\n"), ("b", "0\n")]}, False,
         "upstream module a's build exited 3 with no teq JVM backend summary on its log"),
        ("teq-exit-3-without-its-line", ran, {"*": ran, "jvm1": dict(ran, exit="3\n")}, {}, False, "teq's jvm1 leg's build exited 3 with no teq JVM backend summary"),
        ("teq-cannot-write-tasty-without-the-summary", ran, {"*": ran, "js": dict(ran, exit="1\n", log="<cell>/m/A.scala: cannot write TASTy for x\n")}, {}, False,
         "teq's js leg's build exited 1 with no teq error or TASTy summary"),
        ("scalac-exit-3", dict(refused_b, exit="3\n", log=refused_b["log"] + "3 constructs not supported by the JVM backend\n"), {"*": teq_refused_b}, {}, False, "scalac's build exited 3"),
        ("exit-2-with-a-refusal-line-outside-a-migration-cell", ran, {"*": abi_refused}, migration, False, "teq's jvm1 leg's build exited 2"),
        ("migration--exit-1-with-the-line-only", ran, {"*": abi_refused, "js": {"exit": "1\n", "log": "<refusal>\n"}}, migration, False, "teq's js leg's build exited 1 with no teq error"),
        # a wrong value
        ("wrong-output", ran, {"*": ran, "jvm1": dict(ran, out="22\n")}, {}, False, "NO: 0, `22`"),
        ("refusal-in-another-file", refused_b, {"*": teq_refused_a}, {}, False, "NO:"),
        ("refusal-at-another-line", refused_b, {"*": teq_refused_b3}, {}, False, "NO:"),
        ("refusal-with-no-place", refused_b, {"*": {"exit": "1\n", "log": "error: something\n"}}, {}, False, "teq's jvm1 leg's build exited 1 with no teq error or TASTy summary on its log"),
        ("refused-where-scalac-runs", ran, {"*": teq_refused_b}, {}, False, "NO:"),
        ("migration--builds", ran, {"*": ran}, migration, False, "NO: not refused with the ABI line"),
        ("refused-by-the-jvm-backend-where-scalac-refuses", refused_b, {"*": teq_refused_b, "jvm1": {"exit": "3\n", "log": "1 construct not supported by the JVM backend\n"}}, {}, False, "NO: 3"),
        ("refused-writing-tasty-where-scalac-runs", ran, {"*": ran, "jvm8": {"exit": "1\n", "log": "<cell>/m/A.scala: cannot write TASTy for x\n1 definition cannot be written as TASTy\n"}}, {}, False, "NO: 1"),
        ("migration--refused-otherwise", ran, {"*": abi_refused, "interp": teq_refused_b}, migration, False, "NO: not refused with the ABI line"),
    ]
    cases = []
    for name, scalac, legs, kw, passes, word in one:
        root = os.path.join(tmp, name)
        os.makedirs(root)
        cell(root, name, scalac, legs, **kw)
        cases.append({"name": name, "root": root, "manifest": [name], "passes": passes, "rows": [(name, word)],
                      "version": "9.9.9" if name.startswith("migration--") else None, "ours": "1"})
    # The cells against the manifest: a stray, a duplicate of either kind, a missing cell; each other record good.
    def several(name, cells, manifest, rows):
        root = os.path.join(tmp, name)
        os.makedirs(root)
        for c, kw in cells:
            cell(root, c, ran, {"*": ran}, **kw)
        cases.append({"name": name, "root": root, "manifest": manifest, "passes": False, "rows": rows, "version": None, "ours": None})

    several("stray", [("c", {}), ("s", {})], ["c"], [("s", "a stray"), ("c", "=")])
    several("duplicate-directory", [("c", {}), ("d", {"named": "c"})], ["c", "d"],
            [("d", "a duplicate: its cell.json names c"), ("c", "a duplicate: d's cell.json names this cell too")])
    several("duplicate-in-the-manifest", [("c", {})], ["c", "c"], [("c", "a duplicate: selection.py makes 2 cells")])
    several("missing", [("c", {})], ["c", "gone"], [("gone", "missing: a cell selection.py makes"), ("c", "=")])
    # selection.py's own manifest: one of its cells here, its others missing, and a stray.
    several("selection-py-manifest", [("term--one", {}), ("not-a-cell", {})], None,
            [("term--one", "="), ("term--two--a-up--teq", "missing"), ("not-a-cell", "a stray")])
    return cases


def self_test():
    """The planted cells through the judge: each as expected, its rows printed with their texts."""
    import io
    import shutil
    import tempfile

    tmp = tempfile.mkdtemp(prefix="selection-self-test-")
    bad, cases = [], []
    try:
        cases = planted(tmp)
        for case in cases:
            buf = io.StringIO()
            try:
                rc, _ = judge(case["root"], case["version"], case["ours"], out=buf, manifest=case["manifest"])
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
        print(f"selection self-test: NOT AS EXPECTED: {b}")
    print(f"selection self-test: {len(cases)} cases, {len(cases) - len(bad)} as expected")
    return 1 if bad else 0


def main():
    if sys.argv[1:] == ["--self-test"]:
        return self_test()
    root = sys.argv[1]
    version, ours = (sys.argv[2], sys.argv[3]) if len(sys.argv) > 3 else (None, None)
    rc, _ = judge(root, version, ours)
    return rc


if __name__ == "__main__":
    sys.exit(main())
