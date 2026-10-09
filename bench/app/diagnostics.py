#!/usr/bin/env python3
"""scalac 3.8.4 as the oracle of teq's diagnostics: the same sources and class path compiled by both, their
diagnostics compared as multisets of structured records, the differences against a known list.

  diagnostics.py app <teq> <out dir>
      The application's API side (bench/app/api-scalac-diff.sh, the gate's `app-scalac`): the main lists
      (APP_MODULES, APP_CLASSPATH, APP_FLAGS, APP_SCALAC_OPTIONS) as one program, then the test lists
      (APP_TEST_*) over each compiler's own products of the main ones, in the checkout APP_ROOT, which is read
      and never written; the known list SCALAC_KNOWN.
  diagnostics.py files <teq> <out dir> [--scalac-options "<options>"] [--flags "<flags>"] [--known <file>] <input>...
      The inputs as one program, scope `files` (tests/scalac-oracle.sh).

Each compiler builds its products: teq `compiler build --products --target jvm --std scala-library
--analysis-version 3` over the module directories, its answer's diagnostics the records; scalac through
bench/app/scalac-diagnostics.scala (scala-cli) over the same files, as teq expands the directories, its reporter's
JSON lines the records. A run completes when the compiler exits 0, or 1 explained by its records (an error, or for
teq a warning under --werror, its answer's `ok` false exactly then), and its records are complete: teq's answer
parses and holds as many diagnostics as its stderr's headers, scalac's lines as many as its summary counts. A
timeout, a crash, an unexplained failure, an unparsed answer or a count that disagrees fails the scope, never an
empty comparison.

The conformance pass runs both with warnings enabled: the build's scalacOptions less its suppressions and promotions
(-Wconf, -nowarn, -Werror, -Xfatal-warnings, with any value: `-Werror:false` too) for scalac, its teq flags less
--werror for teq; the options teq ignores (sbt-teq's mapping, the lists' summary) are printed. A record is its
scope, file (relative to the checkout, absolute outside it), severity, category and payload, its places (scalac's
point and its span's start and end, teq's start and end, each a 1-based line and column) and its message, of which
only a place in this run's products (`<out dir>/<scope>.products!`, where teq names an inlined body) is rewritten,
as `<out>/`. The category maps scalac's error codes and teq's messages onto one name where the mapping is tested
(CATEGORIES); the payload is what the message names (the found and required types with a wrapped type's continuation
lines, the missing cases, the symbol) as written, only the layout of a wrapped type joined (a line break and the
next line's indentation as one space), so that a symbol or literal with spaces in it keeps them; any other message
is its own category with the whole message as its payload. Within a scope, file and severity the records pair up:
equal category and payload with teq's start at scalac's point or span start, line and column, is agreement; then,
teq's start on a line of scalac's span, the same category and payload elsewhere is an `anchor` difference, the same
category with another payload a `payload` difference, anything else a `category` difference; the rest is
`scalac-only` or `teq-only`. The production pass compares the exit codes under the build's own options and flags
(`production-exit`); scalac's is read off the conformance pass when what it left out is bare -Werror or
-Xfatal-warnings, under which scalac fails exactly when it warns or errs, and is run again otherwise.

The known list: `cause <tag>  <cause>, <the case that pins it>` names a cause, and
`<scope> <kind> <file>:<line>|- <severity>|- <scalac>|- <teq>|- <count> <tag>[  <note>]` permits a difference:
the digests (sha256, 16 hex) of the complete records of each side, their exit codes for `production-exit`, and
how many times it occurs. A difference no line names fails, a line whose difference does not occur (stale, or
naming nothing compiled) fails, a count other than the line's fails; two lines of one difference, a tag without
its cause and a malformed line are refused. With SCALAC_DRAFT=<file> every difference is written as a line of
that list, its tag the listed one or `?`. <out dir>/differences.txt has every difference with both records whole.
Exits 0 when every scope completes and every difference is listed, 1 otherwise, 2 for a usage or known-list error.
"""
import collections, hashlib, json, os, re, shutil, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
DRIVER = os.path.join(HERE, "scalac-diagnostics.scala")

def sections(*labels):
    """scalac's sections of a message: each label's line, its indented continuation lines joined (a type the
    printer wraps), as a payload field; nothing unless every label is found. A label is scalac's with the spaces
    that align its text (`Found:    `, `Required: `), so that a type whose name begins with spaces keeps them; the
    only other layout that goes is a line break with the next line's indentation, joined by one space. Whatever the
    text holds, a quoted literal or an identifier with spaces in it, stays as scalac wrote it."""
    def extract(message):
        lines, found = message.split("\n"), {}
        for i, l in enumerate(lines):
            for key, label in labels:
                if l.startswith(label) and key not in found:
                    j = i + 1
                    while j < len(lines) and lines[j][:1] in (" ", "\t") and lines[j].strip():
                        j += 1
                    found[key] = " ".join([l[len(label):]] + [c.lstrip(" \t") for c in lines[i + 1:j]])
        return found if len(found) == len(labels) else None
    return extract


# The categories both compilers have, by scalac's error code and by teq's message: scalac's payload is a regex's
# named groups over the message or its sections, teq's a regex's over the whole message.
CATEGORIES = [
    ("type-mismatch", "E007", sections(("found", "Found:    "), ("required", "Required: ")),
     r"type mismatch: found (?P<found>.+), required (?P<required>[^\n]+)"),
    ("exhaustivity", "E029", sections(("missing", "It would fail on pattern case: ")),
     r"match may not be exhaustive; missing: (?P<missing>[^\n]+)"),
    ("already-defined", "E161", r"(?P<name>\S+) is already defined as (?P<existing>[^\n]+)",
     r"(?:(?P<kind>\w+) )?(?P<name>\S+) is already defined(?: as (?P<existing>[^\n]+))?"),
    ("not-member", "E008", r"(?P<member>.+?) is not a member of (?P<owner>[^\n]+)",
     r"(?P<member>.+?) is not a member of (?P<owner>[^\n]+)"),
    ("not-found", "E006", r"Not found: (?P<name>[^\n]+)", r"not found: (?P<name>[^\n]+)"),
    ("unreachable", "E030", r"Unreachable case", r"unreachable case"),
    ("unused", "E198", r"unused (?P<what>[^\n]+)", r"unused (?P<what>[^\n]+)"),
]
# The build's options that suppress or promote warnings, whatever their value (`-Werror:false` too): left out of
# the conformance pass. The production pass's scalac exit is read off the conformance pass only when the options
# left out are bare promotions, so that nothing else changes what scalac says.
SUPPRESSIONS = re.compile(r"^(-Wconf|-nowarn)(:.*)?$")
PROMOTIONS = re.compile(r"^(-Werror|-Xfatal-warnings)(:.*)?$")
BARE_PROMOTIONS = ("-Werror", "-Xfatal-warnings")
KINDS = ("scalac-only", "teq-only", "anchor", "payload", "category", "production-exit")


class Failed(Exception):
    """A run that did not complete: the scope fails with it, never with an empty comparison."""


class Refused(Exception):
    """A usage or a known list the oracle cannot take: exit 2."""


def space(s):
    return re.sub(r"\s+", " ", s).strip()


def categorize(side, code, message):
    for name, scalac_code, scalac_payload, teq_re in CATEGORIES:
        found = None
        if side == "scalac" and code == scalac_code:
            if callable(scalac_payload):
                found = scalac_payload(message)
            else:
                m = re.search(scalac_payload, message)
                found = m and m.groupdict()
        elif side == "teq":
            m = re.fullmatch(teq_re, message, re.S)
            found = m and m.groupdict()
        if found is not None and found is not False:
            return name, {k: v for k, v in found.items() if v is not None}
    return (f"scalac {code or 'uncoded'}" if side == "scalac" else "teq"), {"message": message}


def record(side, scope, root, cwd, out, d, code=""):
    path = d.get("file")
    rel = None
    if path and path != "<library>":
        full = os.path.normpath(path if os.path.isabs(path) else os.path.join(cwd, path))
        rel = os.path.relpath(full, root)
        rel = full if rel.startswith("..") else rel
    elif path:
        rel = path
    # A place in this run's products (`<out dir>/<scope>.products!<source>`, where teq names an inlined body) is
    # written under <out>; nothing else of a message is rewritten.
    message = re.sub(re.escape(out.rstrip("/")) + r"/([\w-]+\.products)!", r"<out>/\1!", d.get("message", ""))
    category, payload = categorize(side, code, message)
    # Where it is: scalac's point, its span's start and end; teq's start and end; each a line and a column.
    at = [(d["line"], d["col"])] if "line" in d else []
    if side == "scalac" and at:
        at += [(d["startLine"], d["startCol"]), (d["endLine"], d["endCol"])]
    elif "endLine" in d:
        at += [(d["endLine"], d["endCol"])]
    return {"side": side, "scope": scope, "file": rel, "line": d.get("line"), "at": at,
            "severity": d.get("severity"), "category": category, "payload": payload, "message": message}


def digest(r):
    if r is None:
        return "-"
    body = {k: r[k] for k in ("category", "payload", "at", "message")}
    return hashlib.sha256(json.dumps(body, sort_keys=True).encode()).hexdigest()[:16]


def expand(path):
    """The files teq reads of an input: the `.scala` files under a directory, its hidden directories left out
    (collect_files in src/main.rs), or the file itself."""
    if not os.path.isdir(path):
        return [path]
    found = []
    for entry in sorted(os.listdir(path)):
        full = os.path.join(path, entry)
        if entry.endswith(".scala") or (not entry.startswith(".") and os.path.isdir(full)):
            found += expand(full)
    return found


def run(args, cwd, out, err, seconds):
    start = time.monotonic()
    try:
        with open(out, "w") as o, open(err, "w") as e:
            code = subprocess.run(args, cwd=cwd, stdout=o, stderr=e, timeout=seconds).returncode
    except subprocess.TimeoutExpired:
        code = "timeout"
    return code, time.monotonic() - start


def tail(path, n=3):
    try:
        return " | ".join(l.rstrip() for l in open(path, errors="replace").read().splitlines()[-n:])
    except OSError:
        return "(no output)"


HEADER = re.compile(r"^(?:\S.*?: )?(error|warning): ")


def teq_run(teq, label, out, cwd, root, scope, inputs, classpath, flags, seconds):
    """teq's build of the inputs into <out>/<label>.products: its exit and records."""
    products = os.path.join(out, label + ".products")
    shutil.rmtree(products, ignore_errors=True)
    answer, log = os.path.join(out, label + ".answer"), os.path.join(out, label + ".teq.log")
    cp = ["--classpath", ":".join(classpath)] if classpath else []
    args = [teq, "compiler", "build", "--products", products, *inputs, *cp, *flags,
            "--std", "scala-library", "--target", "jvm", "--analysis-version", "3"]
    code, took = run(args, cwd, answer, log, seconds)
    if code not in (0, 1):
        raise Failed(f"teq's {label} exits {code} after {took:.0f} s: {tail(log)}")
    lines = [l for l in open(answer).read().splitlines() if l.strip()]
    try:
        a = json.loads(lines[-1])
        items = a["diagnostics"]
        ok = a["ok"]
    except (IndexError, KeyError, ValueError) as e:
        raise Failed(f"teq's {label} gave no answer with diagnostics ({type(e).__name__}): {tail(log)}")
    items = [d for d in items if d.get("severity") in ("error", "warning")]
    headers = sum(1 for l in open(log, errors="replace") if HEADER.match(l))
    if headers != len(items):
        raise Failed(f"teq's {label} printed {headers} diagnostics and answered {len(items)}")
    # A failure is explained by an error, or by a warning under --werror; the answer says ok exactly when teq exits 0.
    errors = sum(1 for d in items if d["severity"] == "error")
    warnings = len(items) - errors
    werror = "--werror" in flags
    if ok is not (code == 0) or (code == 1) != (errors > 0 or (werror and warnings > 0)):
        raise Failed(f"teq's {label} exits {code} with {errors} errors and {warnings} warnings in its answer (ok {str(ok).lower()}"
                     f"{', under --werror' if werror else ''}): {tail(log)}")
    return code, [record("teq", scope, root, cwd, out, d) for d in items], took, products


def scalac_run(label, out, cwd, root, scope, files, classpath, options, seconds):
    """scalac's compile of the files into <out>/<label>.classes: its exit and records."""
    driver_dir = os.path.join(out, "scalac-driver")
    os.makedirs(driver_dir, exist_ok=True)
    shutil.copy(DRIVER, driver_dir)
    classes = os.path.join(out, label + ".classes")
    shutil.rmtree(classes, ignore_errors=True)
    os.makedirs(classes)
    argsfile, jsonl, log = (os.path.join(out, label + x) for x in (".args", ".jsonl", ".scalac.log"))
    with open(argsfile, "w") as f:
        f.write("".join(f'"{p}"\n' for p in files))
    cp = ["-classpath", ":".join(classpath)] if classpath is not None else ["-usejavacp"]
    args = [os.environ.get("SCALA_CLI", "scala-cli"), "run", "--server=false", "--suppress-outdated-dependency-warning", "--jvm", "system",
            "--java-opt", "-Xmx6g", "--java-opt", "-Xss8m", os.path.join(driver_dir, "scalac-diagnostics.scala"), "--",
            jsonl, "-color:never", "-d", classes, *cp, *options, "@" + argsfile]
    if os.path.exists(jsonl):
        os.remove(jsonl)
    code, took = run(args, cwd, log, log + ".stderr", seconds)
    summary = [l for l in open(log, errors="replace") if l.startswith("scalac: ")]
    if code not in (0, 1) or not summary or not os.path.exists(jsonl):
        raise Failed(f"scalac's {label} exits {code} after {took:.0f} s without its summary: {tail(log)} {tail(log + '.stderr')}")
    m = re.match(r"scalac: (\d+) errors, (\d+) warnings", summary[-1])
    items = [json.loads(l) for l in open(jsonl) if l.strip()]
    counted = [d for d in items if d["severity"] in ("error", "warning")]
    errors = sum(1 for d in counted if d["severity"] == "error")
    if not m or (int(m.group(1)), int(m.group(2))) != (errors, len(counted) - errors):
        raise Failed(f"scalac's {label} summary `{summary[-1].strip()}` disagrees with its {len(counted)} records")
    if (code == 1) != (errors > 0):
        raise Failed(f"scalac's {label} exits {code} with {errors} errors")
    return code, [record("scalac", scope, root, cwd, out, d, d.get("code", "")) for d in counted], took, classes


def anchored(a, b):
    """teq's start is scalac's point or its span's start, line and column."""
    return (b["at"][:1] or [None])[0] in ((a["at"][:2]) or [None])


def near(a, b):
    """teq's start is on a line of scalac's span or its point."""
    if not a["at"] or not b["at"]:
        return a["at"] == b["at"]
    lines = [l for l, _ in a["at"]]
    return min(lines) <= b["at"][0][0] <= max(lines)


def compare(scalac, teq):
    """The differences between two scopes' records, each (kind, scalac record, teq record): within a file and a
    severity, the pairs that agree are taken first, then those that differ in the anchor alone, in the payload, in
    the category, each of them with teq's start in scalac's span; the rest is on one side only."""
    groups = collections.defaultdict(lambda: ([], []))
    for r in scalac:
        groups[(r["file"], r["severity"])][0].append(r)
    for r in teq:
        groups[(r["file"], r["severity"])][1].append(r)
    found = []
    for key in sorted(groups, key=lambda k: (k[0] or "", k[1] or "")):
        s, t = (sorted(x, key=lambda r: (r["at"], r["category"], r["message"])) for x in groups[key])
        same = lambda a, b: (a["category"], a["payload"]) == (b["category"], b["payload"])
        for kind, matches in (
            (None, lambda a, b: same(a, b) and anchored(a, b)),
            ("anchor", lambda a, b: same(a, b) and near(a, b)),
            ("payload", lambda a, b: a["category"] == b["category"] and near(a, b)),
            ("category", near),
        ):
            for a in list(s):
                b = next((b for b in t if matches(a, b)), None)
                if b is not None:
                    s.remove(a)
                    t.remove(b)
                    if kind:
                        found.append((kind, a, b))
        found += [("scalac-only", a, None) for a in s] + [("teq-only", None, b) for b in t]
    return found


def key_of(scope, kind, a, b):
    r = a or b
    where = f"{r['file']}:{r['line']}" if r["file"] else "-"
    return (scope, kind, where, r["severity"], digest(a), digest(b))


def load_known(path):
    causes, known = {}, {}
    if not path:
        return causes, known
    for n, l in enumerate(open(path), 1):
        if not l.strip() or l.startswith("#"):
            continue
        w = l.split()
        if w[0] == "cause":
            if len(w) < 3:
                raise Refused(f"{path}:{n}: a cause line is `cause <tag>  <cause>, <case>`")
            causes[w[1]] = l.split(None, 2)[2].strip()
            continue
        if len(w) < 8 or w[1] not in KINDS or not w[6].isdigit():
            raise Refused(f"{path}:{n}: not a line of the known list: {l.strip()}")
        key = tuple(w[:6])
        if key in known:
            raise Refused(f"{path}:{n}: the same difference as line {known[key][2]}")
        known[key] = (int(w[6]), w[7], n)
    for key, (_, tag, n) in known.items():
        if tag not in causes:
            raise Refused(f"{path}:{n}: names the cause {tag}, which no `cause` line gives")
    return causes, known


def describe(r):
    if r is None:
        return "    (none)"
    where = f"{r['file']}:{' '.join(f'{l}:{c}' for l, c in r['at'])}" if r["file"] else "(no position)"
    return f"    {r['side']} {where} {r['severity']} [{r['category']}] {json.dumps(r['payload'], sort_keys=True)}\n      " + r["message"].replace("\n", "\n      ")


def conformance(options):
    kept, dropped, skip = [], [], False
    for i, o in enumerate(options):
        if skip:
            skip = False
            dropped.append(o)
        elif o == "-Wconf" and i + 1 < len(options):
            dropped.append(o)
            skip = True
        elif SUPPRESSIONS.match(o) or PROMOTIONS.match(o):
            dropped.append(o)
        else:
            kept.append(o)
    return kept, dropped


def lines_of(path, comments=False):
    return [l.rstrip("\n") for l in open(path) if l.strip() and not (comments and l.startswith("#"))]


def main():
    if len(sys.argv) < 4 or sys.argv[1] not in ("app", "files"):
        raise Refused(__doc__.split("\n\n")[1])
    mode, teq, out = sys.argv[1], os.path.abspath(sys.argv[2]), os.path.abspath(sys.argv[3])
    os.makedirs(out, exist_ok=True)
    seconds = int(os.environ.get("SCALAC_ORACLE_SECONDS", "1200"))
    scopes = []  # (scope, cwd, inputs, classpath or None, scalac options, teq flags, over)
    if mode == "app":
        need = ["APP_ROOT", "APP_MODULES", "APP_CLASSPATH", "APP_FLAGS", "APP_SCALAC_OPTIONS",
                "APP_TEST_MODULES", "APP_TEST_CLASSPATH", "APP_TEST_FLAGS", "APP_TEST_SCALAC_OPTIONS"]
        missing = [v for v in need if v not in os.environ or (v not in ("APP_FLAGS", "APP_TEST_FLAGS", "APP_ROOT") and not os.path.isfile(os.environ[v]))]
        if missing:
            print(f"scalac oracle: set {', '.join(missing)} (bench/app/app-lists.sh writes them)")
            return 1
        root = os.path.abspath(os.environ["APP_ROOT"])
        known_path = os.environ.get("SCALAC_KNOWN", "")
        for scope, p in (("main", "APP_"), ("test", "APP_TEST_")):
            inputs = [w for l in lines_of(os.environ[p + "MODULES"], True) for w in l.split()]
            # An input is a source directory or a source file (app-lists.sh lists a loose source itself).
            missing = [i for i in inputs if not any(f(os.path.join(root, i)) for f in (os.path.isdir, os.path.isfile))]
            if not os.path.isdir(root) or missing or not inputs:
                where = f" (no {missing[0]})" if missing else ""
                print(f"scalac oracle: no application checkout at {root} with the sources of {os.environ[p + 'MODULES']}{where}")
                return 1
            cp = [os.path.expanduser(l.strip()) for l in lines_of(os.environ[p + "CLASSPATH"])]
            scopes.append((scope, root, inputs, cp, lines_of(os.environ[p + "SCALAC_OPTIONS"]), os.environ[p + "FLAGS"].split(), "main" if scope == "test" else None))
    else:
        rest, opts, flags, known_path = sys.argv[4:], [], [], ""
        while rest and rest[0].startswith("--"):
            if rest[0] == "--scalac-options":
                opts = rest[1].split()
            elif rest[0] == "--flags":
                flags = rest[1].split()
            elif rest[0] == "--known":
                known_path = rest[1]
            else:
                raise Refused(f"diagnostics.py files: unknown option {rest[0]}")
            rest = rest[2:]
        root = os.getcwd()
        scopes.append(("files", root, rest, None, opts, flags, None))
    causes, known = load_known(known_path)

    observed = collections.Counter()
    shown = {}
    status, built, summary = 0, {}, []
    report = open(os.path.join(out, "differences.txt"), "w")
    for scope, cwd, inputs, cp, options, flags, over in scopes:
        files = [os.path.relpath(f, cwd) for i in inputs for f in expand(os.path.join(cwd, i))]
        if not files:
            summary.append(f"{scope}: its inputs hold no source")
            status = 1
            continue
        s_cp, t_cp = cp, cp
        if over:
            if over not in built:
                summary.append(f"{scope}: not compared, {over} did not complete")
                status = 1
                continue
            s_main, t_main = built[over]
            # A product line of the test class path is a main module: the first takes the compiler's own build of
            # the main lists, the others go (bench/app/app-lists.sh).
            def over_products(entries, products):
                placed, result = False, []
                for e in entries:
                    if e.startswith("@"):
                        if not placed:
                            result.append(products)
                            placed = True
                    else:
                        result.append(e)
                return result
            s_cp, t_cp = over_products(cp, s_main), over_products(cp, t_main)
        s_opts, dropped = conformance(options)
        t_flags = [f for f in flags if f != "--werror"]
        try:
            t_code, t_recs, t_took, t_products = teq_run(teq, scope, out, cwd, root, scope, inputs, t_cp, t_flags, seconds)
            s_code, s_recs, s_took, s_classes = scalac_run(scope, out, cwd, root, scope, files, s_cp, s_opts, seconds)
        except Failed as e:
            summary.append(f"{scope}: {e}")
            status = 1
            continue
        built[scope] = (s_classes, t_products)
        diffs = compare(s_recs, t_recs)
        for kind, a, b in diffs:
            key = key_of(scope, kind, a, b)
            observed[key] += 1
            shown.setdefault(key, (a, b))
        # The production pass: the exit codes under the build's own options and flags.
        if all(o in BARE_PROMOTIONS for o in dropped):
            s_prod = 1 if any(r["severity"] == "error" for r in s_recs) or (dropped and s_recs) else 0
        else:
            try:
                s_prod = scalac_run(scope + "-production", out, cwd, root, scope, files, s_cp, options, seconds)[0]
            except Failed as e:
                summary.append(f"{scope}: production pass: {e}")
                status = 1
                continue
        if flags == t_flags:
            t_prod = t_code
        else:
            try:
                t_prod = teq_run(teq, scope + "-production", out, cwd, root, scope, inputs, t_cp, flags, seconds)[0]
            except Failed as e:
                summary.append(f"{scope}: production pass: {e}")
                status = 1
                continue
        if s_prod != t_prod:
            observed[(scope, "production-exit", "-", "-", str(s_prod), str(t_prod))] += 1
        count = lambda rs, sev: sum(1 for r in rs if r["severity"] == sev)
        summary.append(f"{scope}: {len(files)} files; scalac {count(s_recs, 'error')} errors, {count(s_recs, 'warning')} warnings in {s_took:.0f} s; "
                       f"teq {count(t_recs, 'error')} errors, {count(t_recs, 'warning')} warnings in {t_took:.0f} s; {len(diffs)} differences; "
                       f"under the build's options scalac exits {s_prod}, teq {t_prod}"
                       + (f"; left out of the conformance pass: {' '.join(dropped)}" if dropped else ""))

    unlisted, listed, stale, recounted = [], 0, [], []
    for key, n in sorted(observed.items()):
        entry = known.get(key)
        if entry is None:
            unlisted.append(key)
        elif entry[0] != n:
            recounted.append((key, entry[0], n))
        else:
            listed += 1
    for key in sorted(k for k in known if k not in observed):
        stale.append(key)
    for key, n in sorted(observed.items()):
        a, b = shown.get(key, (None, None))
        tag = known.get(key, (0, "?"))[1]
        report.write(f"{' '.join(key)} {n} {tag}\n")
        if key[1] == "production-exit":
            report.write(f"    under the build's options scalac exits {key[4]}, teq {key[5]}\n")
        else:
            report.write(describe(a) + "\n" + describe(b) + "\n")
    report.close()
    if os.environ.get("SCALAC_DRAFT"):
        with open(os.environ["SCALAC_DRAFT"], "w") as d:
            for key, n in sorted(observed.items()):
                a, b = shown.get(key, (None, None))
                note = " / ".join(space(r["message"])[:80] for r in (a, b) if r)
                d.write(f"{' '.join(key)} {n} {known.get(key, (0, '?'))[1]}  {note}\n")
    for line in summary:
        print(line)
    for key in unlisted:
        print(f"not listed: {' '.join(key)} {observed[key]}")
        a, b = shown.get(key, (None, None))
        if key[1] != "production-exit":
            print(describe(a) + "\n" + describe(b))
    for key, want, n in recounted:
        print(f"listed {want} times, seen {n}: {' '.join(key)}")
    for key in stale:
        print(f"listed, and no such difference now: {' '.join(key)}: take it off {known_path}")
    by_cause = collections.Counter(known[k][1] for k in observed if k in known and known[k][0] == observed[k])
    for tag, n in by_cause.most_common():
        print(f"listed {n}, {tag}: {causes[tag]}")
    failed = status or unlisted or stale or recounted
    print(f"scalac oracle: {sum(observed.values())} differences, {listed} listed, {len(unlisted)} not listed, {len(stale)} stale, "
          f"{len(recounted)} counted otherwise{'' if known_path else ' (no known list)'}; {'FAIL' if failed else 'pass'}")
    return 1 if failed else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Refused as e:
        print(e, file=sys.stderr)
        sys.exit(2)
