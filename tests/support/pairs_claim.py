#!/usr/bin/env python3
"""The claim of the pairs part held to scalac (tests/pairs-equality.sh --claim): every cell of the tables the pairs'
proof generates (pairs.tsv: a member of trait O implementing trait T's in five placements; extra.tsv: the design
read's axes), each run by the head on every leg (the check's verdict, the interpreter, JavaScript lean and with
--std scala-library, the JVM) and compared with scalac 3.8.4's recorded answer, the tables' `scalac` and `scalac's
lines` columns. The head's columns are never read.

  pairs_claim.py --head <teq> --claim <table dir>... --out <dir> [--master <teq>] [--rows <file>] [--jobs N] [--only <cell>,...]

A table dir holds cells/<cell> (the sources, scalac's classes of the libraries beside them) and pairs.tsv or
extra.tsv. A cell is built as the generator built it, from its name's placement:
  run (one file)  split (T, O, C apart)  products (T teq's products)  jar (T scalac's classes)
  twolibs (T scalac's classes, O teq's products over them)  bothscalac and cscalac (the library scalac's classes)
  bothprod and cprod (the library teq's products)
A leg not scalac's fails, but in a row the Ready names (--rows, default tests/support/pairs_rows.tsv: program, leg,
family, reason; a cell's program is `cells/<cell>`), where the head must print master's answer; a listed row the head
now answers as scalac fails too. Each cell is built in a copy under <out>/cells (the table dirs are only read).
Writes <out>/claim.tsv (cell, leg, verdict, detail), prints each failure and the
totals; exits 1 when one fails."""
import collections, concurrent.futures, os, shutil, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def arg(name, default=None):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


def args(name):
    return [sys.argv[i + 1] for i, a in enumerate(sys.argv) if a == name]


HEAD, MASTER = arg("--head"), arg("--master")
# the last --out: tests/pairs-equality.sh passes the equality's arguments and then the claim's own
OUT = (args("--out") or [None])[-1] or sys.exit("pairs_claim.py: --out <dir> is needed (each cell is built in a copy there)")
JOBS = int(arg("--jobs", "4"))
SL = os.path.join(os.environ.get("COURSIER_CACHE", os.path.expanduser("~/.cache/coursier/v1")),
                  "https/repo1.maven.org/maven2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar")
ENV = dict(os.environ, LC_ALL="C.UTF-8", TZ="UTC", TEQ_NO_CACHE="1", TEQ_THREADS=os.environ.get("TEQ_THREADS", "1"))
ROWS = {}
with open(arg("--rows", os.path.join(ROOT, "tests/support/pairs_rows.tsv")), encoding="utf-8") as fh:
    for line in fh:
        line = line.rstrip("\n")
        if line and not line.startswith("#"):
            f = line.split("\t")
            ROWS[(f[0], f[1])] = f[2] + ": " + (f[3] if len(f) > 3 else "")


def run(cmd, cwd, timeout=180, env=None):
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, env=env or ENV, cwd=cwd, errors="replace")
    except subprocess.TimeoutExpired:
        return 1, "TIMEOUT"
    return r.returncode, (r.stdout.strip() if r.returncode == 0 else "FAILED " + (r.stdout + r.stderr).strip()[-200:])


def placement(cell):
    """(program files, library sources by name or None, how each library is given: scalac's classes or products)."""
    parts = cell.split("-")
    prov = parts[-1]
    if len(parts) == 4:  # pairs.tsv: impl-other-where-prov
        files = {"run": ["p.scala"], "split": ["t.scala", "o.scala", "c.scala"], "products": ["oc.scala"], "jar": ["oc.scala"],
                 "twolibs": ["c.scala"]}[prov]
        return files, prov
    return ["main.scala"], prov  # extra.tsv: member-place-prov


def products(teq, d, side, name, src, mode, deps, env):
    """A library in teq's products for one leg's mode ('lean', 'sl', 'jvm'), over the class path deps."""
    p = os.path.join(d, f"claim-{side}-{name}-{mode}")
    os.makedirs(p, exist_ok=True)
    flags = {"lean": ["compiler", "check"], "sl": ["compiler", "check", "--std", "scala-library"], "jvm": ["compiler", "build", "--target", "jvm"]}[mode]
    cp = ":".join(x for x in ([SL] if mode != "lean" else []) + deps if x)
    run([teq] + flags + ["--no-cache"] + (["--classpath", cp] if cp else []) + ["--products", p, os.path.join(d, src)], d, 300, env)
    return p


def libraries(teq, d, side, prov, env):
    """The library class path of each leg's mode for the cell's placement."""
    none = {"lean": "", "sl": "", "jvm": ""}
    if prov in ("run", "split"):
        return none
    if prov == "jar":
        return {m: os.path.join(d, "scalac-T") for m in none}
    if prov == "products":
        return {m: products(teq, d, side, "T", "T.scala", m, [], env) for m in none}
    if prov == "twolibs":
        t = os.path.join(d, "scalac-T")
        return {m: t + ":" + products(teq, d, side, "O", "O.scala", m, [t], env) for m in none}
    if prov in ("bothscalac", "cscalac"):
        return {m: os.path.join(d, "scalac-lib") for m in none}
    return {m: products(teq, d, side, "lib", "lib.scala", m, [], env) for m in none}  # bothprod, cprod


def legs(teq, d, side, files, prov, env):
    lib = libraries(teq, d, side, prov, env)
    lean = ["--classpath", lib["lean"]] if lib["lean"] else []
    res = {}
    rc, out = run([teq, "compiler", "check", "--no-cache"] + lean + files, d, 180, env)
    res["check"] = "accepted" if rc == 0 else "refused"
    if rc != 0:
        return res
    res["interp"] = run([teq, "interp"] + lean + files, d, 180, env)[1]
    js = os.path.join(d, f"claim-{side}.js")
    rc, out = run([teq, "compiler", "build", "--no-cache"] + lean + files + ["-o", js], d, 180, env)
    res["js"] = run(["node", js], d, 180, env)[1] if rc == 0 else "BUILD " + out
    slcp = SL + (":" + lib["sl"] if lib["sl"] else "")
    jss = os.path.join(d, f"claim-{side}-sl.js")
    rc, out = run([teq, "compiler", "build", "--no-cache", "--std", "scala-library", "--classpath", slcp] + files + ["-o", jss], d, 180, env)
    res["js-sl"] = run(["node", jss], d, 180, env)[1] if rc == 0 else "BUILD " + out
    jcp = SL + (":" + lib["jvm"] if lib["jvm"] else "")
    jar = os.path.join(d, f"claim-{side}.jar")
    rc, out = run([teq, "compiler", "build", "--no-cache", "--target", "jvm", "--classpath", jcp] + files + ["-o", jar], d, 180, env)
    res["jvm"] = run(["java", "-XX:-UsePerfData", "-cp", jar + ":" + jcp, "TeqMain"], d, 180, env)[1] if rc == 0 else "BUILD " + out
    return res


def one(job):
    cell, src, scalac_ok, scalac_lines = job
    # the cell's sources and scalac's classes copied under --out: the claim's directories are only read
    d = os.path.join(OUT, "cells", cell)
    if not os.path.isdir(d):
        # the cell's own sources and scalac's class directories, whole (their packages and class files); nothing else
        shutil.copytree(src, d, ignore=lambda at, names: [n for n in names if not (n.endswith(".scala") or n.startswith("scalac"))] if os.path.samefile(at, src) else [])
    files, prov = placement(cell)
    files = [os.path.join(d, f) for f in files]
    env_head = dict(ENV)
    if os.environ.get("TEQ_PAIRS_CONTROL"):
        # a control acts under the pairs' dump alone (crate::tir::pairs_control): the head writes it beside the cell
        env_head["TEQ_PAIRS_DUMP"] = os.path.join(d, "claim-pairs.txt")
    h = legs(HEAD, d, "head", files, prov, env_head)
    m = legs(MASTER, d, "master", files, prov, dict(ENV, TEQ_PAIRS_CONTROL="")) if MASTER else {}
    name = "cells/" + cell
    want = scalac_lines if scalac_ok else None
    rows = []
    for leg in ("check", "interp", "js", "js-sl", "jvm"):
        if leg == "check":
            got, exp = h.get("check"), ("accepted" if scalac_ok else "refused")
        else:
            if not scalac_ok or h.get("check") != "accepted":
                continue
            got, exp = h.get(leg, "").replace("\n", " | "), want
        mine = m.get(leg, "").replace("\n", " | ") if m else None
        key = (name, leg)
        if got == exp:
            rows.append((cell, leg, "ROW NOW SCALAC'S", f"listed ({ROWS[key]}) and the head now answers as scalac") if key in ROWS else (cell, leg, "scalac's", ""))
        elif key in ROWS:
            if mine is None:
                rows.append((cell, leg, "row", ROWS[key] + " (no master given: its answer not compared)"))
            elif got == mine:
                rows.append((cell, leg, "row", ROWS[key]))
            else:
                rows.append((cell, leg, "NOT MASTER'S", f"a row ({ROWS[key]}): master {mine[:100]!r} head {got[:100]!r}"))
        else:
            rows.append((cell, leg, "NOT SCALAC'S", f"scalac {exp[:100]!r} head {got[:100]!r}"))
    return rows


def jobs():
    out = []
    for t in args("--claim"):
        tsv = next((os.path.join(t, f) for f in ("pairs.tsv", "extra.tsv") if os.path.exists(os.path.join(t, f))), None)
        with open(tsv, encoding="utf-8") as fh:
            header = fh.readline().rstrip("\n").split("\t")
            ci, si, li = header.index("cell"), header.index("scalac"), header.index("scalac's lines")
            for line in fh:
                f = line.rstrip("\n").split("\t")
                out.append((f[ci], os.path.join(t, "cells", f[ci]), f[si] == "True", f[li]))
    return out


def main():
    os.makedirs(OUT, exist_ok=True)
    work = jobs()
    if arg("--only"):
        work = [w for w in work if w[0] in arg("--only").split(",")]
    counts, bad = collections.Counter(), 0
    lines = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=JOBS) as ex:
        for rows in ex.map(one, work):
            for cell, leg, verdict, detail in rows:
                counts[verdict] += 1
                lines.append(f"{cell}\t{leg}\t{verdict}\t{detail}")
                if verdict.isupper():
                    bad += 1
                    print(f"cells/{cell}\t{leg}\t{verdict}\t{detail}", flush=True)
    if OUT:
        with open(os.path.join(OUT, "claim.tsv"), "w", encoding="utf-8") as fh:
            fh.write("".join(l + "\n" for l in sorted(lines)))
    listed = sum(1 for k in ROWS if k[0].startswith("cells/"))
    print(f"claim: cells {len(work)}; " + ", ".join(f"{v} {n}" for v, n in sorted(counts.items())) + f"; rows listed {listed}; failing {bad}")
    sys.exit(1 if bad else 0)


main()
