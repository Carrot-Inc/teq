#!/usr/bin/env python3
"""tests/kept-body-lists/record.py: writes tests/kept-body-lists/expected.tsv from scalac 3.8.4 alone (it never runs
teq): every program gen.py makes is compiled whole (A.scala and B.scala together, each program in its own package) by
scalac 3.8.4's compiler jars from the coursier cache, in batches from which the programs scalac refuses are dropped one
batch after another, and each compiled program's main runs on the JVM. A program's observable is its run's output, or
`REFUSED` where scalac refuses it (the error's first line after a tab, information only).

Needs java and, in the coursier cache (COURSIER_CACHE, else ~/.cache/coursier/v1), scala3-compiler_3 3.8.4 and its
dependencies, which `scala-cli compile -S 3.8.4` fetches. Run it when gen.py changes; the suite's run (run.py) only
reads its output."""
import concurrent.futures
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
EXPECTED = os.path.join(HERE, "expected.tsv")
CACHE = os.environ.get("COURSIER_CACHE", os.path.expanduser("~/.cache/coursier/v1"))
M2 = os.path.join(CACHE, "https", "repo1.maven.org", "maven2")
JARS = ["org/scala-lang/scala3-compiler_3/3.8.4/scala3-compiler_3-3.8.4.jar", "org/scala-lang/scala3-interfaces/3.8.4/scala3-interfaces-3.8.4.jar",
        "org/scala-lang/tasty-core_3/3.8.4/tasty-core_3-3.8.4.jar", "org/scala-lang/scala3-library_3/3.8.4/scala3-library_3-3.8.4.jar",
        "org/scala-lang/modules/scala-asm/9.9.0-scala-1/scala-asm-9.9.0-scala-1.jar", "org/scala-sbt/compiler-interface/1.10.7/compiler-interface-1.10.7.jar",
        "org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar"]
LIB = os.path.join(M2, "org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar")
JOBS = int(os.environ.get("JOBS", "8"))


def cell(text):
    """As run.py holds a cell: an object's identity and an anonymous class's suffix taken out, one line."""
    text = re.sub(r"\$anon\$[0-9a-z_]+", "$anon", re.sub(r"@[0-9a-f]+\b", "@<identity>", text))
    return text.rstrip("\n").replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n")


def dotc(files, out):
    if os.path.isdir(out):
        shutil.rmtree(out)
    os.makedirs(out)
    cp = ":".join(os.path.join(M2, j) for j in JARS)
    p = subprocess.run(["java", "-XX:-UsePerfData", "-Xss16m", "-Xmx3g", "-cp", cp, "dotty.tools.dotc.Main", "-color:never", "-classpath", LIB, "-d", out] + files,
                       capture_output=True, text=True, timeout=3600)
    return p.returncode, p.stdout + p.stderr


def main():
    missing = [j for j in JARS if not os.path.exists(os.path.join(M2, j))]
    if missing:
        sys.exit("record.py: not in the coursier cache: " + " ".join(missing))
    with tempfile.TemporaryDirectory(prefix="kb-lists-record-") as work:
        subprocess.run([sys.executable, os.path.join(HERE, "gen.py"), work], stdout=subprocess.DEVNULL, check=True)
        names = open(os.path.join(work, "rows.txt")).read().split()
        rows = os.path.join(work, "rows")
        files = lambda n: [os.path.join(rows, n, "A.scala"), os.path.join(rows, n, "B.scala")]
        left, refused = list(names), {}
        classes = os.path.join(work, "classes")
        for it in range(12):
            rc, log = dotc([f for n in left for f in files(n)], classes)
            if rc == 0:
                break
            bad = {}
            for line in log.splitlines():
                m = re.match(r"-- .*?(?:Error|error).*?/rows/([^/]+)/[AB]\.scala:(\d+)", line)
                if m:
                    bad.setdefault(m.group(1), line)
                elif "while compiling:" in line:
                    sys.exit(f"record.py: scalac crashed ({line.strip()}): no program's answer is recorded")
            if not bad:
                sys.exit("record.py: a batch failed with no program named:\n" + log[-2000:])
            for n, line in bad.items():
                # the header without the work directory: `[E172] Type Error: A.scala:7:24`
                refused[n] = re.sub(r"\S*/rows/[^/]+/", "", line).strip().strip("-").strip()
            left = [n for n in left if n not in bad]
        else:
            sys.exit("record.py: no clean batch in 12")

        def run(n):
            p = subprocess.run(["java", "-XX:-UsePerfData", "-Xss32m", "-cp", classes + ":" + LIB, f"g_{n}.Main"], capture_output=True, text=True, timeout=120)
            return n, p.returncode, p.stdout, p.stderr

        with concurrent.futures.ThreadPoolExecutor(JOBS) as ex:
            ran = {n: (rc, out, err) for n, rc, out, err in ex.map(run, left)}
    lines = ["# program\tscalac 3.8.4's observable (its run's output; REFUSED, then after a tab the error's header, information)\n"]
    failed = []
    for n in names:
        if n in refused:
            lines.append(f"{n}\tREFUSED\t{refused[n]}\n")
            continue
        rc, out, err = ran[n]
        if rc != 0:
            failed.append(f"{n}: the JVM run exits {rc}: {err.strip().splitlines()[:1]}")
        lines.append(f"{n}\t{cell(out)}\n")
    if failed:
        sys.exit("record.py: " + "; ".join(failed[:10]))
    with open(EXPECTED, "w") as fh:
        fh.write("".join(lines))
    print(f"record.py: {len(names)} programs, scalac compiles {len(names) - len(refused)} and refuses {len(refused)}; expected.tsv written")


if __name__ == "__main__":
    main()
