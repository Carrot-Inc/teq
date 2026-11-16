#!/usr/bin/env python3
"""scalac's own diagnostics of every test of the board's neg- and warn-kind rows: the rows' oracles, which the
board's judge (tests/board/dotty.py) holds teq's diagnostics to. An oracle is tests/board/dotty/<row>.oracle.jsonl:
a head line (the scalac version, the corpus commit, the JDK, the jars, the captures compared), then a JSON line a
test in the runner's order. This file makes them, compares two captures and validates them; no suite runs a
capture, and an oracle moves only with the scalac version or the corpus.

  python3 tests/board/oracle.py run --jdk <JDK home> --work <dir> [--corpus <dotty at the tag>] [--scalac 3.8.4]
          [--threads 4] [--limit 300] [--only ROW,ROW] [--sjs V,V,V]
                                         the compilations, into a new directory (a capture each)
  python3 tests/board/oracle.py read --work <dir> [--corpus <dir>]
                                         that capture's entries, <work>/oracle.jsonl
  python3 tests/board/oracle.py compare <work A> <work B>
                                         two captures: their raw renderings unmasked, then their entries
  python3 tests/board/oracle.py stability --texts <file> [--out <dir>] <work 1> <work 2> ...
                                         the first capture's entries, each `stable` where every other capture has
                                         the same, as the rows' files in <dir> (tests/board/dotty); the unstable
                                         entries' texts, capture by capture, in <file>
  python3 tests/board/oracle.py validate [--dir <dir>] [--row ROW,ROW] [--corpus <dir>] [--work <dir>]
                                         the rows' files against the corpus, without a compiler; exit 1 where an
                                         entry does not reproduce its test's .check. The board's runner runs it
                                         over the rows it is about to judge (dotty.py), so every gate line of a
                                         neg- or warn-kind row does

The tests are the runner's own (dotty.py's `tests_of`, `tool_args`, the groups in order; rows.py's flags), the
corpus is the runner's (`dotty.corpus()`: $SCALA3_TAG, else beside $SCALA3) unless --corpus names it, the JDK is an
argument, and scalac's class path and the libraries are resolved by coursier from their versions (`cs fetch`; the
launcher is $COURSIER, by default `cs`). A work directory is the tool's own: `run` makes it and refuses one that
holds anything.

HOW A TEST IS COMPILED (vulpix, compiler/test/dotty/tools/vulpix/ParallelTesting.scala `compile`, at the tag): each
group of the test in order, `-classpath <the row's library>:<products>`, the row's flags, the group's own tool
arguments (dotty.tool_args of the group's files, without the options the runner derives from a source's imports:
scalac reads the import itself), `-d <products>`, the group's files (.scala and .java, `JAVA_ONLY` files left out,
the other platform's left out), one products directory for all groups (a chain step of rows.py into its own,
against the steps it names); after a group holding .java files, javac over them (`-encoding UTF-8 -classpath <the
same> -d <products>`, the group's javac arguments; `SCALA_ONLY` files left out). The working directory is the
corpus root, so the paths are vulpix's. A test whose `// test: -jvm N` the JDK does not meet is skipped, as vulpix
skips it, and one with a group for another scalac (`_c3.0.0`) is not captured. A scala-js row: `-scalajs` and the
Scala.js library jars (TestConfiguration.scalaJSClasspath: javalib, scalalib, library; --sjs gives their versions,
by default those of the tag's build) before scala-library (the published scalalib holds .sjsir only). One JVM
(OracleCapture.java, compiled here by the JDK's javac), the tag's .jvmopts (-Xss2m -Xms1024m -Xmx8192m
-XX:MaxInlineLevel=35), --threads compilations at a time.

THE DIAGNOSTICS are recorded by the reporter AS THEY ARE REPORTED (OracleCapture.java): an entry a diagnostic, in
order, across the groups. Each is `{severity, file, line, column, id, message}`:
  - severity: error / warning / migration warning (a MigrationWarning) / info, from the diagnostic's level and class;
  - a diagnostic with a position (the position as vulpix keys the marks: `pos.nonInlined`, the call site of an
    inlined one): its rendering (vulpix's console rendering of that one diagnostic) read by the board's reader of
    scalac's boxes (harness.scalac_diagnostics: the file's name, the line, the column from the caret, 1-based, and
    the message's lines, the `longer explanation` trailer left out), so that a .check read by the same reader
    reproduces; where the reader takes no column or no message of a box (its own source line's gutter set right,
    ` 7 |`), the entry keeps the reader's reading, flagged `"reader": "blind"`, with the reporter's own place in
    `"at": [line, start column]`; the judge compares such a diagnostic by severity, file and line and never passes
    a test on it;
  - a diagnostic without a position: file, line and column null, the message its rendered lines;
  - where the reader's severity of a box is not the diagnostic's own, the entry keeps the diagnostic's and says the
    reader's (`reader severity`): a .check of that test then fails the validation;
  - id: scalac's own code (`E007`), null where the message has none;
  - an info is an entry only where vulpix's console prints it (under -Ylog: TestConsoleReporter.isDiagnosticRelevant);
    the echoes of -Ycheck (`checking <file> after phase <p>`) and the like are left out.
An entry is `valid: true` only where every group's scalac run and every javac step completed (a run that threw or
ran over the limit, a javac that could not be launched, a test skipped: `valid: false`, with `why`): such an entry
is no answer. A javac that ran and refused (as vulpix's would) leaves the entry valid, with a note.

THE DIGEST (dotty.digest): sha256 over the test's files in the runner's order (`Test.files`), of each its path
from the corpus (UTF-8, `/`), a NUL, its length in bytes in decimal, a NUL and its bytes. The judge gives no
answer from an entry whose digest is not the test's. Outside the digest: the row's flags (rows.py, which a capture
imports), the files scalac reads beside the test's own (a -sourcepath directory, a class path directory of the
corpus) and the corpus commit, which the head line names and the judge holds to its own.

THE MASK (dotty.MASK, applied by `read` and by the judge alike): an identity hash after a class name, `@` and one
to eight hex digits after a word character, is written `@<hash>`.

STABILITY: scalac does not repeat every diagnostic from run to run (the order of a hash set's iteration, the depth
at which it catches an overflow), so an oracle is made from several captures, each into a work directory of its
own with clean products, one of them at one thread: the first directory given to `stability` is the oracle, and an
entry that differs, masked, in any other is `stable: false`, no answer for the judge. More captures find more.

VALIDATION (`validate`, no compiler): a row's file must hold an entry for each of the row's tests, once, of the
head's scalac, saying its `valid`, its `stable` and its `digest`, the digest that of the test's files. Every valid
entry of a test with a .check must reproduce it: the .check
read by the board's reader gives the entry's placed diagnostics, in order, and the entry's place-less lines are
lines of the .check, in order. It also holds each entry to the test's marks as dotty's NegTest and WarnTest would
(dotty.held_to_marks), where every diagnostic's file is one of the test's by name. With --work (the capture's own
directory) the marks are checked for every entry by the reporter's paths, and each .check is compared line for line
with the console renderings in order, as vulpix's FileDiff would; a difference there in lines the reader does not
read (an inline trace quoting the library's source) is printed and fails nothing.
"""
import argparse
import collections
import json
import os
import subprocess
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import dotty as D  # noqa: E402
import rows as R  # noqa: E402
from harness import scalac_diagnostics  # noqa: E402  (dotty.py puts tests/scala3-typing on the path)

JVMOPTS = ["-Xss2m", "-Xms1024m", "-Xmx8192m", "-XX:MaxInlineLevel=35"]
RECIPE = "tests/board/oracle.py (its docstring: how a test is compiled, the diagnostics, the digest, the mask, stability)"


def cs_fetch(coords, classpath=False, intransitive=False):
    cmd = [os.environ.get("COURSIER", "cs"), "fetch"] + (["--classpath"] if classpath else []) + (["--intransitive"] if intransitive else []) + coords
    r = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
    if r.returncode != 0:
        sys.exit(f"cs fetch {' '.join(coords)} failed: {r.stderr.strip()[-400:]}")
    return r.stdout.strip()


def jdk_major(jdk):
    with open(os.path.join(jdk, "release"), encoding="utf-8") as fh:
        for l in fh:
            if l.startswith("JAVA_VERSION="):
                v = l.split("=", 1)[1].strip().strip('"')
                return int(v.split(".")[0])
    sys.exit("no JAVA_VERSION in the JDK's release file")


def corpus_of(a):
    return os.path.abspath(a.corpus) if a.corpus else D.corpus()


def tests_of_rows(corpus, only=None):
    out = []
    for row in R.rows():
        if row.kind not in ("neg", "warn") or row.cannot:
            continue
        if only and row.name not in only:
            continue
        out.extend(D.tests_of(row, corpus))
    return out


def plan(t, corpus, products, lib, sjs, jdk):
    """The test's compilations: [(options, scalac args, javac args)], or None and why (vulpix skips it)."""
    rel = lambda f: os.path.relpath(f, corpus)  # noqa: E731
    allargs = D.tool_args(t.files, imports=False)
    other = set(allargs[1].get("scala-js" if t.row.platform == "jvm" else "jvm", []))
    cpl = (sjs + lib) if t.row.platform == "scala-js" else lib
    extra = ["-scalajs"] if t.row.platform == "scala-js" else []
    steps = t.steps or [{} for _ in t.groups]
    comps, outs = [], []
    for i, group in enumerate(t.groups):
        args, _, error = D.tool_args([f for f in group if f.endswith(D.SRC)], imports=False)
        spec = D.options_jvm(args.get("test", []))
        if spec and spec != "unknown" and not D.jvm_fits(spec, jdk):
            return None, f"skipped as vulpix skips it: the test is for JDK {spec[0]}{'+' if spec[1] else ''}, this is {jdk}"
        if spec == "unknown":
            return None, "dotty: unknown test option: " + " ".join(args.get("test", []))
        if error:
            return None, "dotty refuses the test's tool arguments: " + error
        if getattr(t, "compilers", None) and t.compilers[i]:
            return None, f"a group for scalac {t.compilers[i]}: vulpix compiles it with that compiler, not captured here"
        files = [f for f in group if f.endswith(D.SRC) and "JAVA_ONLY" not in os.path.basename(f) and f not in other]
        if not files:
            outs.append(None)
            continue
        if t.steps:
            d = f"{products}.{i}"
            step = steps[i]
            cp = [outs[j] for j in step.get("against", []) if outs[j]] + [os.path.join(corpus, "tests", c) for c in step.get("classpath", [])] + cpl
            flags = step.get("flags", t.row.flags)
        else:
            d, cp, flags = products, cpl, t.flags
        outs.append(d)
        os.makedirs(d, exist_ok=True)
        opts = list(flags) + extra + args.get("scalac", [])
        classpath = ":".join(cp + [d])
        javas = [f for f in group if f.endswith(".java") and "SCALA_ONLY" not in os.path.basename(f)]
        javac = (["-encoding", "UTF-8", "-classpath", classpath, "-d", d] + args.get("javac", []) + [rel(f) for f in javas]) if javas else []
        comps.append((opts, ["-classpath", classpath] + opts + ["-d", d] + [rel(f) for f in files], javac))
    return comps, None


def severity(level, kind):
    if kind == "MigrationWarning":
        return "migration warning"
    return {0: "info", 1: "warning", 2: "error"}.get(level, f"level {level}")


def entry_diagnostics(raw):
    """The oracle's entries of one test from its raw records, in the reporter's order: the console's ones (`console`:
    vulpix prints a WARNING or above, an info of a phase -Ylog names); a placed one read by the board's reader from
    its rendering, masked; a box the reader takes no column or message of (its own gutter set right) keeps the
    reader's reading, flagged `reader: "blind"` with the reporter's own place (`at`: line and start column,
    1-based)."""
    out = []
    for d in raw:
        if not d["console"]:
            continue
        sev = severity(d["level"], d["kind"])
        text = D.MASK.sub("@<hash>", d["rendered"])
        if d["file"] is None:
            lines = [l.rstrip() for l in text.replace("\r", "").split("\n")]
            while lines and not lines[-1]:
                lines.pop()
            out.append({"severity": sev, "file": None, "line": None, "column": None, "id": d["id"], "message": lines})
            continue
        read = scalac_diagnostics(text)
        if len(read) == 1:
            s, f, line, col, msg = read[0]
            e = {"severity": sev, "file": f, "line": line, "column": col, "id": d["id"], "message": list(msg)}
            if col == 0 or not msg:
                e["reader"] = "blind"
                e["at"] = [d["line"] + 1, d["startColumn"] + 1]
            if s != sev:
                e["reader severity"] = s
            out.append(e)
        else:
            out.append({"severity": sev, "file": os.path.basename(d["file"]), "line": d["line"] + 1, "column": d["startColumn"] + 1,
                        "id": d["id"], "message": [], "reader": f"read {len(read)} boxes in this rendering"})
    return out


def resolve(a):
    scalac_cp = cs_fetch([f"org.scala-lang:scala3-compiler_3:{a.scalac}"], classpath=True)
    lib = cs_fetch([f"org.scala-lang:scala-library:{a.scalac}"], intransitive=True).splitlines()
    jv, sl, li = a.sjs.split(",")
    # TestConfiguration.scalaJSClasspath, in its order: javalib, scalalib, library; each artifact's own jar alone
    sjs = []
    for org_name, v in (("org.scala-js:scalajs-javalib", jv), ("org.scala-js:scalajs-scalalib_2.13", sl), ("org.scala-js:scalajs-library_2.13", li)):
        name = org_name.split(":")[1]
        jars = [j for j in cs_fetch([f"{org_name}:{v}"], intransitive=True).splitlines() if os.path.basename(j).startswith(f"{name}-")]
        if len(jars) != 1:
            sys.exit(f"cs fetch {org_name}:{v} gave {jars}: one jar expected")
        sjs += jars
    lib = [j for j in lib if os.path.basename(j).startswith("scala-library-")]
    return scalac_cp, lib, sjs


def run(a):
    corpus = corpus_of(a)
    jdk = jdk_major(a.jdk)
    javac = os.path.join(a.jdk, "bin", "javac")
    java = os.path.join(a.jdk, "bin", "java")
    scalac_cp, lib, sjs = resolve(a)
    work = os.path.abspath(a.work)
    marker = os.path.join(work, ".oracle-capture")
    if os.path.exists(work) and os.listdir(work):
        sys.exit(f"{work} is not empty: `run` takes an empty or a new directory (a capture each)")
    os.makedirs(work, exist_ok=True)
    with open(marker, "w") as fh:
        fh.write("made by tests/board/oracle.py run\n")
    classes = os.path.join(work, "classes")
    os.makedirs(classes)
    r = subprocess.run([javac, "-cp", scalac_cp, "-d", classes, os.path.join(HERE, "OracleCapture.java")], capture_output=True, text=True, timeout=300)
    if r.returncode != 0:
        sys.exit("javac of OracleCapture.java failed: " + r.stderr[-800:])
    products = os.path.join(work, "products")
    raw = os.path.join(work, "raw")
    os.makedirs(raw)
    tests = tests_of_rows(corpus, set(a.only.split(",")) if a.only else None)
    index, man = [], []
    for n, t in enumerate(tests):
        tid = f"t{n:05d}"
        comps, skip = plan(t, corpus, os.path.join(products, tid), lib, sjs, jdk)
        index.append({"id": tid, "row": t.row.name, "test": t.name, "options": [c[0] for c in comps] if comps else [], "skip": skip})
        if comps:
            man.append(tid + "\t" + "\x1e".join("\x1f".join(c[1]) + ("\x1d" + "\x1f".join(c[2]) if c[2] else "") for c in comps))
    with open(os.path.join(work, "manifest.tsv"), "w", encoding="utf-8") as fh:
        fh.write("\n".join(man) + "\n")
    with open(os.path.join(work, "index.json"), "w", encoding="utf-8") as fh:
        json.dump({"scalac": a.scalac, "corpus": corpus_commit(corpus), "jdk": jdk, "threads": a.threads,
                   "jars": {"scala-library": [os.path.basename(x) for x in lib], "scala-js": [os.path.basename(x) for x in sjs]},
                   "tests": index}, fh)
    cmd = [java] + JVMOPTS + ["-cp", classes + ":" + scalac_cp, "OracleCapture", os.path.join(work, "manifest.tsv"), raw,
                              str(a.threads), str(a.limit), javac]
    with open(os.path.join(work, "capture.log"), "w") as log:
        r = subprocess.run(cmd, cwd=corpus, stdout=log, stderr=subprocess.STDOUT, timeout=24 * 3600)
    if r.returncode != 0:
        sys.exit(f"the capture JVM ended with {r.returncode}: see {work}/capture.log")
    print(f"{len(index)} tests; {len(man)} compiled into {work}")


def corpus_commit(corpus):
    return subprocess.run(["git", "-C", corpus, "rev-parse", "HEAD"], capture_output=True, text=True, timeout=30).stdout.strip()


def read(a):
    corpus = corpus_of(a)
    work = os.path.abspath(a.work)
    with open(os.path.join(work, "index.json"), encoding="utf-8") as fh:
        meta = json.load(fh)
    tests = {(t.row.name, t.name): t for t in tests_of_rows(corpus)}
    raw = os.path.join(work, "raw")
    out = a.out or os.path.join(work, "oracle.jsonl")
    with open(out, "w", encoding="utf-8") as fh:
        fh.write(json.dumps({"head": True, "scalac": meta["scalac"], "corpus": meta["corpus"], "jdk": meta["jdk"],
                             "threads": meta["threads"], "jars": meta["jars"], "recipe": RECIPE,
                             "digest": "sha256 over Test.files in order: relpath UTF-8, NUL, len decimal ASCII, NUL, bytes",
                             "mask": D.MASK.pattern + " -> @<hash>"}) + "\n")
        for i in meta["tests"]:
            t = tests[(i["row"], i["test"])]
            e = {"row": i["row"], "test": i["test"], "digest": D.digest(corpus, t.files), "scalac": meta["scalac"], "jdk": meta["jdk"],
                 "options": i["options"], "valid": True}
            if i["skip"]:
                e.update(valid=False, why=i["skip"], diagnostics=[])
            else:
                st = os.path.join(raw, i["id"] + ".status")
                if not os.path.exists(st):
                    e.update(valid=False, why="no capture (the JVM did not finish the test)", diagnostics=[])
                else:
                    with open(st, encoding="utf-8") as sh:
                        steps = [l.split("\t") for l in sh.read().splitlines()]
                    bad = [s for s in steps if s[2] != "ok" and not (s[1] == "javac" and s[2].startswith("exit "))]
                    notes = [f"group {s[0]} javac {s[2]}: {s[3][:300]}" for s in steps if s[1] == "javac" and s[2].startswith("exit ")]
                    e["diagnostics"] = entry_diagnostics(raw_records(raw, i["id"]))
                    if bad:
                        e.update(valid=False, why="; ".join(f"group {s[0]} {s[1]}: {s[2]} {s[3][:300]}" for s in bad))
                    if notes:
                        e["notes"] = notes
            fh.write(json.dumps(e, ensure_ascii=False) + "\n")
    print(f"written {out}")


def raw_records(raw, tid):
    with open(os.path.join(raw, tid + ".jsonl"), encoding="utf-8") as fh:
        return [json.loads(l) for l in fh if l.strip()]


def load(path):
    """A file of entries after a head line: (the head, the entries by row and test, in the file's order)."""
    head, out = None, collections.OrderedDict()
    with open(path, encoding="utf-8") as fh:
        for l in fh:
            e = json.loads(l)
            if e.get("head"):
                head = e
            else:
                out[(e["row"], e["test"])] = e
    return head, out


def load_rows(d, only=None):
    """The rows' files of a directory (of the rows named, where any is): (the heads by row, every entry by row and
    test, the entries a file holds once more: each as the file's row, the entry's row and test)."""
    heads, out, twice = {}, collections.OrderedDict(), []
    for row in R.rows():
        path = os.path.join(d, row.name + ".oracle.jsonl")
        if os.path.exists(path) and (not only or row.name in only):
            heads[row.name] = None
            with open(path, encoding="utf-8") as fh:
                for l in fh:
                    if not l.strip():
                        continue
                    e = json.loads(l)
                    if e.get("head"):
                        heads[row.name] = e
                    elif (e["row"], e["test"]) in out:
                        twice.append((row.name, e["row"], e["test"]))
                    else:
                        out[(e["row"], e["test"])] = e
    return heads, out, twice


def write_rows(d, head, entries):
    """The entries as the rows' files of `d`, <row>.oracle.jsonl: a row's entries in the order given after its head
    line, which names the row and the jars of its class path alone (a row's file is replaced by a capture of that
    row). Only the rows the entries hold are written."""
    platform = {r.name: r.platform for r in R.rows()}
    by = collections.OrderedDict()
    for e in entries:
        by.setdefault(e["row"], []).append(e)
    for row, es in by.items():
        h = {"head": True, "row": row}
        for k, v in head.items():
            if k == "jars":
                v = {name: jars for name, jars in v.items() if name != "scala-js" or platform[row] == "scala-js"}
            if k not in ("head", "row"):
                h[k] = v
        with open(os.path.join(d, row + ".oracle.jsonl"), "w", encoding="utf-8") as fh:
            fh.write(json.dumps(h, ensure_ascii=False) + "\n")
            for e in es:
                fh.write(json.dumps(e, ensure_ascii=False) + "\n")
    return list(by)


def compare(a):
    """Two captures (work directories): first their raw records UNMASKED, rendering by rendering (every test whose two
    differ, with the first differing span); then every span the mask rewrites (in the first); then their entries
    (`read`'s output in each: <work>/oracle.jsonl), entry by entry."""
    with open(os.path.join(a.a, "index.json"), encoding="utf-8") as fh:
        ia = json.load(fh)["tests"]
    with open(os.path.join(a.b, "index.json"), encoding="utf-8") as fh:
        ib = {(i["row"], i["test"]): i for i in json.load(fh)["tests"]}
    n_raw, masked = 0, collections.Counter()
    for i in ia:
        k = (i["row"], i["test"])
        j = ib.get(k)
        fa = os.path.join(a.a, "raw", i["id"] + ".jsonl")
        fb = os.path.join(a.b, "raw", j["id"] + ".jsonl") if j else None
        ra = raw_records(os.path.join(a.a, "raw"), i["id"]) if os.path.exists(fa) else []
        rb = raw_records(os.path.join(a.b, "raw"), j["id"]) if fb and os.path.exists(fb) else []
        for r in ra:
            for m in D.MASK.finditer(r["rendered"]):
                masked[r["rendered"][max(0, m.start() - 30):m.end()].split("\n")[-1]] += 1
        if ra != rb:
            n_raw += 1
            print(f"== raw {k[0]}/{k[1]}: {len(ra)} || {len(rb)} records")
            for x, y in zip(ra, rb):
                if x != y:
                    for f in x:
                        if x.get(f) != y.get(f):
                            sa, sb = str(x.get(f)), str(y.get(f))
                            p = next((q for q in range(min(len(sa), len(sb))) if sa[q] != sb[q]), min(len(sa), len(sb)))
                            print(f"   {f}: ...{sa[max(0, p - 40):p + 40]!r} || ...{sb[max(0, p - 40):p + 40]!r}")
                    break
    print(f"{n_raw} tests whose raw records differ (unmasked)")
    print("spans the mask rewrites in the first capture (the text before each hash):")
    for s, n in masked.most_common():
        print(f"  {n}\t{s}")
    A, B = load(os.path.join(a.a, "oracle.jsonl"))[1], load(os.path.join(a.b, "oracle.jsonl"))[1]
    diff = [k for k in sorted(set(A) | set(B)) if json.dumps(A.get(k), sort_keys=True) != json.dumps(B.get(k), sort_keys=True)]
    print(f"the two captures' entries, masked: {len(diff)} differ" + (": " + ", ".join(f"{k[0]}/{k[1]}" for k in diff[:40]) if diff else ""))


def stability(a):
    """The first capture's entries, every one marked `stable`: false where any other capture's entry (masked)
    differs from it, written as the rows' files; and the texts: each unstable test's first differing diagnostic,
    whole, in each capture."""
    if len(a.works) < 2:
        sys.exit("stability: two captures at least (an entry is stable only where another capture has the same)")
    loaded = [load(os.path.join(w, "oracle.jsonl")) for w in a.works]
    head, caps = loaded[0][0], [c for _, c in loaded]
    base = caps[0]
    unstable = {k for c in caps[1:] for k in set(base) | set(c) if json.dumps(base.get(k), sort_keys=True) != json.dumps(c.get(k), sort_keys=True)}
    head["stability"] = f"{len(a.works)} captures compared (masked): an entry differing in any is `stable: false`"
    head["captures"] = [h["threads"] for h, _ in loaded]
    entries = []
    for k, e in base.items():
        e = dict(e)
        e["stable"] = k not in unstable
        entries.append(e)
    rows = write_rows(a.out, head, entries)
    with open(a.texts, "w", encoding="utf-8") as fh:
        for k in sorted(unstable):
            fh.write(f"==== {k[0]}/{k[1]}\n")
            ds = [c.get(k, {}).get("diagnostics", []) for c in caps]
            n = max(len(d) for d in ds)
            i = next((i for i in range(n) if len({json.dumps(d[i] if i < len(d) else None) for d in ds}) > 1), 0)
            fh.write(f"(counts of diagnostics per capture: {[len(d) for d in ds]}; the first that differs: number {i + 1})\n")
            for w, d in zip(a.works, ds):
                x = d[i] if i < len(d) else None
                fh.write(f"-- {os.path.basename(w.rstrip('/'))}: " + ("nothing" if x is None else f"{x['severity']} {x['file']}:{x['line']}:{x['column']} {x['id']}") + "\n")
                if x:
                    fh.write("\n".join("   " + l for l in x["message"]) + "\n")
    print(f"{len(unstable)} entries unstable; written {len(rows)} rows' files into {a.out}, the texts into {a.texts}")


def marked(t, dicts):
    """What dotty's own test would say of scalac's diagnostics: None where the marks hold."""
    if t.row.kind == "neg":
        return D.held_to_marks(None, t, dicts, "error", D.ERROR_MARK, ("nopos", "anypos"), "error")
    if [d for d in dicts if d["severity"] == "error"]:
        return "an error where warnings are marked"
    return D.held_to_marks(None, t, dicts, "warning", D.WARN_MARK, ("nopos",), "warning")


def validate(a):
    corpus = corpus_of(a)
    only = set(a.row.split(",")) if a.row else None
    heads, E, twice = load_rows(a.dir, only)
    tests = {(t.row.name, t.name): t for t in tests_of_rows(corpus, only)}
    raw = ids = None
    if a.work:
        raw = os.path.join(os.path.abspath(a.work), "raw")
        with open(os.path.join(a.work, "index.json"), encoding="utf-8") as fh:
            ids = {(i["row"], i["test"]): i["id"] for i in json.load(fh)["tests"]}
    c = collections.Counter()
    failed, noted = [], []
    for row, h in heads.items():
        if h is None or h.get("corpus") != D.TAG_COMMIT or h.get("scalac") != D.SCALAC:
            failed.append(f"{row}: the head line is not scalac {D.SCALAC}'s at the corpus commit {D.TAG_COMMIT}")
    for k in tests:
        if k not in E:
            failed.append(f"{k[0]}/{k[1]}: no entry")
    for at, row, test in twice:
        failed.append(f"{row}/{test}: an entry twice (in {at}'s file)")
    for k, e in E.items():
        name = f"{k[0]}/{k[1]}"
        t = tests.get(k)
        if t is None:
            failed.append(f"{name}: an entry of no test of the row")
            continue
        c["entries"] += 1
        # what the judge asks of an entry before it compares: said outright, and of this row's capture
        for key, kind in (("valid", bool), ("stable", bool), ("digest", str)):
            if not isinstance(e.get(key), kind):
                failed.append(f"{name}: the entry does not say its `{key}`")
        head = heads.get(k[0]) or {}
        if e.get("scalac") != head.get("scalac"):
            failed.append(f"{name}: the entry is scalac {e.get('scalac')}'s, its file's head says {head.get('scalac')}")
        if isinstance(e.get("digest"), str) and e["digest"] != D.digest(corpus, t.files):
            failed.append(f"{name}: the digest is not the test's files'")
        if e.get("valid") is not True:
            c["not valid"] += 1
            noted.append(f"{name}: not valid: {e.get('why', '')}")
            continue
        c["valid"] += 1
        c["unstable"] += e.get("stable") is not True
        # the marks: by the capture's own paths where its directory is given, else by the name of a test's file
        rawd = [r for r in raw_records(raw, ids[k]) if r["console"]] if raw else None
        by = collections.defaultdict(list)
        for f in t.files:
            by[os.path.basename(f)].append(f)
        dicts = []
        for n, d in enumerate(e["diagnostics"]):
            path = None
            if d["file"] is not None:
                path = os.path.join(corpus, rawd[n]["file"]) if rawd else by[d["file"]][0] if len(by[d["file"]]) == 1 else ""
            dicts.append({"severity": d["severity"], "file": path, "line": d["line"], "col": d["column"], "message": "\n".join(d["message"])})
        if any(d["file"] == "" for d in dicts):
            c["marks not checked"] += 1
            noted.append(f"{name}: the marks are not checked without --work: a diagnostic's file is none of the test's by name, or two of them")
        else:
            why = marked(t, dicts)
            c["marks hold" if not why else "marks fail"] += 1
            if why:
                failed.append(f"{name}: scalac's diagnostics do not hold the test's marks: {why}")
        chk = D.check_file(t)
        if not chk:
            continue
        with open(chk, encoding="utf-8", errors="replace") as fh:
            text = D.MASK.sub("@<hash>", fh.read())
        c["with a .check"] += 1
        want = [d for d in scalac_diagnostics(text) if d[0] != "info"]
        got = [(d["severity"], d["file"], d["line"], d["column"], tuple(d["message"])) for d in e["diagnostics"]
               if d["severity"] != "info" and d["file"] is not None]
        if want == got:
            c["read reproduces"] += 1
        else:
            failed.append(f"{name}: the .check read by the reader is not the entry's placed diagnostics")
        exp = [l.rstrip() for l in D.lines_of(text)]
        loose = [l.rstrip() for d in e["diagnostics"] if d["file"] is None for l in d["message"] if l.strip()]
        if loose:
            c["with place-less lines"] += 1
            at = 0
            for l in loose:
                at = next((q + 1 for q in range(at, len(exp)) if exp[q] == l), None)
                if at is None:
                    failed.append(f"{name}: the .check does not hold the entry's place-less line in its order: {l[:100]}")
                    break
            else:
                c["place-less lines held"] += 1
        if rawd:
            seq = []
            for r in rawd:
                seq += D.lines_of(D.MASK.sub("@<hash>", r["rendered"]))
            want_lines = [l.rstrip("\r") for l in D.lines_of(text)]
            if seq == want_lines:
                c["line for line"] += 1
            else:
                q = next((q for q in range(max(len(seq), len(want_lines))) if (seq[q] if q < len(seq) else None) != (want_lines[q] if q < len(want_lines) else None)), 0)
                noted.append(f"{name}: line {q + 1} of the .check is not the console's: {(want_lines[q] if q < len(want_lines) else None)!r:.150} || {(seq[q] if q < len(seq) else None)!r:.150}")
    for line in noted + ["FAIL " + f for f in failed]:
        print(line)
    print(f"oracle: {c['entries']} entries of {len(heads)} rows, {c['valid']} valid ({c['unstable']} of them unstable), {c['not valid']} not; "
          f"{c['read reproduces']} of {c['with a .check']} .check files reproduce through the reader, {c['place-less lines held']} of "
          f"{c['with place-less lines']} with place-less lines hold them in order" + (f", {c['line for line']} line for line" if raw else "")
          + f"; the marks hold in {c['marks hold']} of {c['marks hold'] + c['marks fail']}" + (f" ({c['marks not checked']} not checked without --work)" if c["marks not checked"] else "")
          + f"; {len(failed)} failed")
    return 1 if failed else 0


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("run")
    for f in ("--jdk", "--work"):
        c.add_argument(f, required=True)
    c.add_argument("--corpus")
    c.add_argument("--scalac", default=D.SCALAC)
    c.add_argument("--threads", type=int, default=4)
    c.add_argument("--limit", type=int, default=300)
    c.add_argument("--only")
    c.add_argument("--sjs", default="1.20.2,3.8.4,1.20.2", help="javalib,scalalib,library versions (the tag's sbt-scalajs is 1.20.2)")
    r = sub.add_parser("read")
    r.add_argument("--work", required=True)
    r.add_argument("--corpus")
    r.add_argument("--out")
    p = sub.add_parser("compare")
    p.add_argument("a")
    p.add_argument("b")
    st = sub.add_parser("stability")
    st.add_argument("--out", default=os.path.join(HERE, "dotty"))
    st.add_argument("--texts", required=True)
    st.add_argument("works", nargs="+")
    v = sub.add_parser("validate")
    v.add_argument("--dir", default=os.path.join(HERE, "dotty"))
    v.add_argument("--row", help="only these rows' files (ROW,ROW)")
    v.add_argument("--corpus")
    v.add_argument("--work")
    a = ap.parse_args()
    return {"run": run, "read": read, "compare": compare, "stability": stability, "validate": validate}[a.cmd](a)


if __name__ == "__main__":
    sys.exit(main())
