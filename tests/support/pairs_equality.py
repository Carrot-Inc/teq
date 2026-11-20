#!/usr/bin/env python3
"""The equality of a class's pairs (tests/pairs-equality.sh): master's outputs against the head's, partitioned by the
head's own record of each class's pairs (`TEQ_PAIRS_DUMP`, `Worker::class_pairs`).

  pairs_equality.py --master <teq> --head <teq> --out <dir> [--cases] [--board <scala3 checkout>] [--dir <d>]...
                    [--legs js,jvm] [--jobs N] [--limit N] [--rows <file>] [--claimed <file>] [--only <program>,...]

The programs: tests/cases (each .scala and each directory), with --board the rows of tests/board/dotty/run.tsv
(the JVM) and run-scalajs.tsv (JavaScript) that pass, and every program of each --dir (a .scala alone, or a
directory of them). Each program is built for its legs by both binaries, the same command; the head writes the
pairs of every class of the program. A program none of whose classes has a pair the record keeps (a pair whose
overriding member the class inherits and is not abstract: the enumeration's `kept` lines, read apart from the stored
record) is held to master byte for byte: its JavaScript file, every class file of its jar. A program of the claim
with one (scalac's classes beside it, or a case tests/support/pairs_claimed.txt names) is run on every leg whatever
its bytes and held to its expectation (scalac's run, the case's .expected): master's answer is accepted only in a
row the Ready names (--rows, default tests/support/pairs_rows.tsv: program, leg, family, reason), where the head
must print master's answer, and a listed row whose head now prints the expectation fails too. Any other program
with one is held to its value: master's lines or its expectation's (its suite holds it to the expectation with
the suite's pending list). The changed outputs are listed by what changed them: a JVM class whose methods differ by a bridge (ACC_BRIDGE), JavaScript lines that
answer a name by calling another (a bridge), read a stored member through its method (an accessor) or compute a
parameter's default from `this` (a default getter). Anything else is unexplained.

Prints a line per program that breaks a rule and the totals; exits 1 when one does."""
import concurrent.futures, os, re, subprocess, sys, zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def arg(name, default=None):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


def args(name):
    return [sys.argv[i + 1] for i, a in enumerate(sys.argv) if a == name]


MASTER, HEAD, OUT = arg("--master"), arg("--head"), arg("--out")
LEGS = arg("--legs", "js,jvm").split(",")
JOBS = int(arg("--jobs", "4"))
LIMIT = int(arg("--limit", "0"))
REUSE = "--reuse" in sys.argv
HOME = os.path.expanduser("~")
SL = os.path.join(os.environ.get("COURSIER_CACHE", HOME + "/.cache/coursier/v1"),
                  "https/repo1.maven.org/maven2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar")
ENV = dict(os.environ, LC_ALL="C.UTF-8", TZ="UTC", TEQ_NO_CACHE="1", TEQ_THREADS=os.environ.get("TEQ_THREADS", "1"))


def run(cmd, cwd, timeout=200, env=None):
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, env=env or ENV, cwd=cwd, errors="replace")
        return r.returncode, r.stdout + r.stderr
    except subprocess.TimeoutExpired:
        return "timeout", ""


sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pairs_programs as pp
lines_of = pp.lines_of

# The rows the Ready names (tests/support/pairs_rows.tsv): (program, leg) -> reason. In a program with a kept pair,
# a leg not the expectation's is accepted there alone, and only where the head prints master's answer.
ROWS = {}
for line in lines_of(arg("--rows", os.path.join(ROOT, "tests/support/pairs_rows.tsv"))):
    if line and not line.startswith("#"):
        f = line.split("\t")
        ROWS[(f[0], f[1])] = f[2] + ": " + (f[3] if len(f) > 3 else "")


CLAIMED = set(l for l in lines_of(arg("--claimed", os.path.join(ROOT, "tests/support/pairs_claimed.txt"))) if l and not l.startswith("#"))


def claim(name, files):
    """Whether the program is the claim's: scalac's classes beside it (a table's cell, a review's program), or a case
    the part names (tests/support/pairs_claimed.txt). Those are held to their expectation whatever their bytes; the
    other programs' expectations are their suites' (tests/run.sh, run_jvm.sh, run_interp.sh, with their pending lists)."""
    return name in CLAIMED or os.path.isdir(os.path.join(os.path.dirname(files[0]), "scalac"))


def kept_pair(dump):
    """Whether the head's enumeration of the program's classes has a pair the record's rule keeps (`kept` lines):
    the partition, read apart from the stored record, so that a pair lost from the store leaves its program held
    to its expectation."""
    return os.path.exists(dump) and any(len(r) >= 12 and r[11] == "kept" for r in (l.split("\t") for l in lines_of(dump)))


def held(name, leg, h, m, want):
    """A leg of a program with a kept pair: run, whatever its bytes, and held to its expectation (the case's
    .expected, the test's .check, scalac's run of the classes beside it). A row the Ready names is accepted where
    the head prints master's answer; a listed row that now prints the expectation is a list to correct."""
    if want is None:
        return "scalac refuses", "no run of scalac's to hold it to: the claim (--claim) holds the verdict"
    # a row of the program's verdict (`check`: teq refuses what scalac accepts, or the reverse) covers each of its
    # legs, none of which has a value of its own
    key = (name, leg) if (name, leg) in ROWS else (name, "check")
    if h[0] == 0 and h[1].splitlines() == want:
        if key in ROWS:
            return "ROW NOW EXPECTED", f"listed as a row ({ROWS[key]}) and the head now prints the expectation"
        return "value ok", "the expectation's"
    if key in ROWS:
        if h == m:
            return "row", ROWS[key]
        return "VALUE", f"a row ({ROWS[key]}) where the head is not master's: master {str(m[1])[:100]!r} head {str(h[1])[:100]!r}"
    return "VALUE", f"not the expectation: master {str(m[1])[:100]!r} head {str(h[1])[:100]!r} want {' | '.join(want)[:100]!r}"


def build(teq, leg, files, flags, out, dump=None):
    env = dict(ENV)
    if dump:
        env["TEQ_PAIRS_DUMP"] = dump
    if leg == "js":
        cmd = [teq, "compiler", "build", "--no-cache"] + files + ["-o", out] + flags
    else:
        cmd = [teq, "compiler", "build", "--no-cache", "--target", "jvm", "--classpath", SL] + files + ["-o", out] + flags
    rc, text = run(cmd, os.path.dirname(out), 300, env)
    if rc == 1 and "several entry points" in text:
        rc, text = run(cmd + ["--main", "Test"], os.path.dirname(out), 300, env)
    return rc


def execute(leg, out):
    if leg == "js":
        js = out if os.path.exists(out) else out[:-3] + ".mjs"
        return run(["node", js], os.path.dirname(out), 60)
    main = "Test" if zip_has(out, "Test.class") else "TeqMain"
    return run(["java", "-XX:-UsePerfData", "-cp", out + ":" + SL, main], os.path.dirname(out), 60)


def zip_has(path, entry):
    try:
        return entry in zipfile.ZipFile(path).namelist()
    except Exception:
        return False


def zip_entries(path):
    with zipfile.ZipFile(path) as z:
        return {n: z.read(n) for n in z.namelist()}


def javap_methods(jar, cls):
    """{declaration with its descriptor: whether ACC_BRIDGE} of the class's methods (javap -v: a declaration is
    indented by two, its descriptor and flags follow it)."""
    rc, text = run(["javap", "-p", "-v", "-cp", jar, cls[:-6].replace("/", ".")], "/", 60)
    methods, last = {}, None
    for l in text.splitlines():
        if re.match(r"^  [^ #].*\(.*\).*;$", l):
            last = l.strip().rstrip(";")
        elif last and l.strip().startswith("descriptor:"):
            last = last + " " + l.strip()[len("descriptor:"):].strip()
            methods[last] = False
        elif last and l.strip().startswith("flags:"):
            methods[last] = "ACC_BRIDGE" in l
            last = None
    return methods


def javap_code(jar, cls):
    """[(declaration with its descriptor, whether ACC_BRIDGE, its code with the constant pool's indices left out)] of the
    class's methods in their order (javap -p -c -v)."""
    rc, text = run(["javap", "-p", "-c", "-v", "-cp", jar, cls[:-6].replace("/", ".")], "/", 60)
    methods, cur = [], None
    for l in text.splitlines():
        if re.match(r"^  [^ #].*\(.*\).*;$", l):
            cur = [l.strip().rstrip(";"), False, []]
            methods.append(cur)
        elif cur and l.strip().startswith("descriptor:") and " (" not in cur[0]:
            cur[0] = cur[0] + " " + l.strip()[len("descriptor:"):].strip()
        elif cur and l.strip().startswith("flags:"):
            cur[1] = "ACC_BRIDGE" in l
        elif cur and re.match(r"^\s+\d+: ", l):
            cur[2].append(re.sub(r"\s+", " ", re.sub(r"#\d+(\.#?\d+)?", "#", re.sub(r"^\s+\d+: ", "", l))).strip())
    return methods


def jvm_class_kind(mo, ho, cls):
    """What changed one class of the jar: `bridge` (methods added, all ACC_BRIDGE, none removed), `bridge target` (the
    methods are the same and those whose code differs are bridges), `order` (the same methods with the same code in
    another order), or an unexplained line."""
    mm, hm = javap_code(mo, cls), javap_code(ho, cls)
    md, hd = {m[0]: m for m in mm}, {m[0]: m for m in hm}
    added = [k for k in hd if k not in md]
    removed = [k for k in md if k not in hd]
    if removed:
        return f"UNEXPLAINED class {cls}: removed {removed[:3]} added {added[:3]}"
    if any(not hd[k][1] for k in added):
        return f"UNEXPLAINED class {cls}: added {[k for k in added if not hd[k][1]][:3]}"
    body = [k for k in md if md[k][2] != hd[k][2]]
    if any(not (md[k][1] or hd[k][1]) for k in body):
        return f"UNEXPLAINED class {cls}: the code of {[k for k in body if not (md[k][1] or hd[k][1])][:3]} changed"
    if added:
        return "bridge"
    if body:
        return "bridge target"
    if [m[0] for m in mm] != [m[0] for m in hm]:
        return "order"
    return "constants"


def js_hunk_kind(removed, added):
    """What changed one hunk of a program's JavaScript, by its shape: a default getter (`f$default$1`, its body, a
    parameter computing it from `this`), a bridge (a method answering one name by calling another on `this`), an
    accessor (a stored member read through a method: the field renamed `$f`, the method `f()` beside it, the reads
    called), or None."""
    lines = removed + added
    if any("$default$" in l for l in lines):
        return "default getter"
    bridge = re.compile(r"^\s*[\w$]+\((\.\.\.\$a|[^)]*)\)\s*\{\s*return this\.[\w$]+\(.*\);\s*\}$")
    if added and not removed and all(bridge.match(l) for l in added):
        return "bridge"
    accessor_method = re.compile(r"^\s*[\w$]+\(\)\s*\{\s*return this\.\$[\w$]+;\s*\}$")

    def norm(l):
        l = l.replace('"$', '"').replace("get $", "get ").replace("this.$", "this.")
        return re.sub(r"\.([A-Za-z_][\w$]*)\(\)", r".\1", l)

    kept = [norm(l) for l in added if not accessor_method.match(l)]
    if removed != added and [norm(l) for l in removed] == kept:
        return "accessor"
    return None


def js_changes(ma, he):
    import difflib
    out = []
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, ma, he, autojunk=False).get_opcodes():
        if tag == "equal":
            continue
        kind = js_hunk_kind(ma[i1:i2], he[j1:j2])
        out.append(kind or "UNEXPLAINED " + " | ".join((["-" + l for l in ma[i1:i2]] + ["+" + l for l in he[j1:j2]])[:6])[:240])
    return out


def scalac_expectation(files):
    """For a table's program: scalac's run of the classes beside it (the table's `scalac` directory), if any."""
    d = os.path.join(os.path.dirname(files[0]), "scalac")
    if not os.path.isdir(d):
        return None
    # scalac's classes run over its two libraries, from the coursier cache as the suites find scala-library
    scala3 = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(SL))), "scala3-library_3", "3.8.4", "scala3-library_3-3.8.4.jar")
    rc, text = run(["java", "-XX:-UsePerfData", "-cp", d + ":" + scala3 + ":" + SL, "run"], d, 60)
    return text.splitlines() if rc == 0 else None


def interp_leg(name, d, files, flags, expected):
    """The interpreter's leg: both binaries run the program (`teq interp`); a difference is held to the expectation
    (the case's .expected, a table's scalac run). The interpreter has no bytes: its partition is the head's record."""
    dump = os.path.join(d, "pairs-interp.txt")
    if os.path.exists(dump):
        os.unlink(dump)
    env = dict(ENV)
    m = run([MASTER, "interp"] + files + flags, d, 120, env)
    env["TEQ_PAIRS_DUMP"] = dump
    h = run([HEAD, "interp"] + files + flags, d, 120, env)
    want = lines_of(expected) if expected and os.path.exists(expected) else scalac_expectation(files)
    kept = kept_pair(dump)
    if kept and claim(name, files):
        verdict, detail = held(name, "interp", h, m, want)
        return ("interp", verdict, detail)
    if m == h:
        return ("interp", "value ok" if kept else "equal", "unchanged" if kept else "no inherited pair")
    ok = want is not None and h[0] == 0 and h[1].splitlines() == want
    return ("interp", "value ok" if ok else "VALUE", "no inherited pair; head is the expectation" if ok else f"no inherited pair; master {str(m[1])[:120]!r} head {str(h[1])[:120]!r}")


def program(p):
    name, files, flags, legs, expected = p
    d = os.path.join(OUT, name.replace("/", "_"))
    os.makedirs(d, exist_ok=True)
    rows = []
    if "interp" in LEGS and "js" in legs:
        rows.append(interp_leg(name, d, files, flags, expected.get("js")))
    for leg in legs:
        if leg == "interp":
            continue
        dump = os.path.join(d, f"pairs-{leg}.txt")
        if os.path.exists(dump) and not REUSE:
            os.unlink(dump)
        ext = ".js" if leg == "js" else ".jar"
        mo, ho = os.path.join(d, "master" + ext), os.path.join(d, "head" + ext)
        if REUSE and os.path.exists(mo) and os.path.exists(ho):
            mrc = hrc = 0
        else:
            mrc = build(MASTER, leg, files, flags, mo)
            hrc = build(HEAD, leg, files, flags, ho, dump)
        if mrc != 0 or hrc != 0:
            if (mrc == 0) != (hrc == 0):
                rows.append((leg, "BUILD", f"master exit {mrc}, head exit {hrc}"))
            continue
        inherited = kept_pair(dump)
        if leg == "js":
            ma, he = lines_of(mo if os.path.exists(mo) else mo[:-3] + ".mjs"), lines_of(ho if os.path.exists(ho) else ho[:-3] + ".mjs")
            same = ma == he
        else:
            me, hent = zip_entries(mo), zip_entries(ho)
            same = me == hent
        if not inherited:
            rows.append((leg, "equal" if same else "UNEQUAL", "no inherited pair"))
            continue
        # A program with a kept pair: run on the leg whatever its bytes and held to its expectation; what changed
        # where its bytes did.
        claimed = claim(name, files)
        if same and not claimed:
            rows.append((leg, "value ok", "unchanged"))
            continue
        mv, hv = execute(leg, mo), execute(leg, ho)
        want = lines_of(expected[leg]) if expected.get(leg) and os.path.exists(expected[leg]) else scalac_expectation(files)
        if claimed:
            verdict, detail = held(name, leg, hv, mv, want)
        else:
            ok = hv == mv or (want is not None and hv[0] == 0 and hv[1].splitlines() == want)
            verdict, detail = ("value ok", "master's or the expectation's") if ok else ("VALUE", f"master {str(mv[1])[:100]!r} head {str(hv[1])[:100]!r}")
        if same:
            rows.append((leg, verdict, "unchanged; " + detail))
            continue
        changes = []
        if leg == "js":
            changes = js_changes(ma, he)
        else:
            for cls in sorted(set(me) | set(hent)):
                if me.get(cls) == hent.get(cls):
                    continue
                if cls not in me or cls not in hent:
                    changes.append(f"UNEXPLAINED class {cls} only on one side")
                    continue
                changes.append(jvm_class_kind(mo, ho, cls))
        rows.append((leg, verdict, ("; ".join(sorted(set(changes))) or "unchanged") + "; " + detail))
    return name, rows


def main():
    os.makedirs(OUT, exist_ok=True)
    progs = []
    if "--cases" in sys.argv:
        progs += list(pp.case_programs(ROOT, LEGS))
    if arg("--board"):
        progs += list(pp.board_programs(ROOT, arg("--board"), LEGS))
    for d in args("--dir"):
        progs += list(pp.dir_programs(d, LEGS))
    if arg("--only"):
        progs = [p for p in progs if p[0] in arg("--only").split(",")]
    if LIMIT:
        progs = progs[:LIMIT]
    totals = {}
    bad = 0
    with open(os.path.join(OUT, "results.tsv"), "w") as f, concurrent.futures.ThreadPoolExecutor(max_workers=JOBS) as ex:
        for name, rows in ex.map(program, progs):
            for leg, verdict, detail in rows:
                part = "own pairs only" if detail.startswith("no inherited pair") else "inherited pair"
                totals[(part, leg, verdict)] = totals.get((part, leg, verdict), 0) + 1
                f.write(f"{name}\t{leg}\t{part}\t{verdict}\t{detail}\n")
                broken = verdict in ("UNEQUAL", "VALUE", "BUILD", "ROW NOW EXPECTED") or "UNEXPLAINED" in detail
                if broken:
                    bad += 1
                    print(f"{name}\t{leg}\t{verdict}\t{detail[:300]}", flush=True)
    for k in sorted(totals):
        print(f"{k[0]}\t{k[1]}\t{k[2]}\t{totals[k]}")
    print(f"programs {len(progs)}; broken {bad}")
    sys.exit(1 if bad else 0)


main()
