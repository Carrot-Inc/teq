#!/usr/bin/env python3
"""initdump.py: scalac 3.8.4's view of the initialization checker's input, the oracle of tests/init-view.sh.

  python3 tests/init-view/initdump.py --out <dir> [--option <scalac option>]... <file.scala>...
      compiles the files with scalac (InitDump.scala's driver, with -Wsafe-init and -Ysafe-init-global), against
      scala-library 3.8.4, and writes <dir>/pickler.dump and <dir>/initChecker.dump (InitDump.scala's header says
      what they hold), <dir>/scalac.txt (scalac's diagnostics as it prints them) and <dir>/exit (its exit status)

InitDump.scala is compiled once per its content and the scalac version into out/init-view/initdump-<digest>/ (the
compiler with its own class path resolved by coursier, as tests/board/oracle.py resolves it: `cs fetch`, the
launcher $COURSIER, by default `cs`), then run in a JVM per call. The working directory of the run is the first
file's directory, so the paths scalac prints are the files' names as the board's corpus gives them."""
import argparse
import hashlib
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
SCALAC = "3.8.4"
JVMOPTS = ["-Xss2m", "-Xms256m", "-Xmx4096m", "-XX:-UsePerfData"]


def cs_fetch(coords, classpath=False, intransitive=False):
    cmd = ([os.environ.get("COURSIER", "cs"), "fetch"] + (["--classpath"] if classpath else [])
           + (["--intransitive"] if intransitive else []) + coords)
    r = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
    if r.returncode != 0:
        sys.exit(f"cs fetch {' '.join(coords)} failed: {r.stderr.strip()[-400:]}")
    return r.stdout.strip().splitlines()[-1]


def compiled():
    """The directory of InitDump's classes and scalac's class path, compiling InitDump.scala where its digest is new."""
    src = os.path.join(HERE, "InitDump.scala")
    with open(src, "rb") as fh:
        digest = hashlib.sha256(fh.read() + SCALAC.encode()).hexdigest()[:16]
    scalac_cp = cs_fetch([f"org.scala-lang:scala3-compiler_3:{SCALAC}"], classpath=True)
    base = os.path.join(REPO, "out", "init-view", f"initdump-{digest}")
    classes = os.path.join(base, "classes")
    done = os.path.join(base, "compiled")
    if not os.path.exists(done):
        os.makedirs(classes, exist_ok=True)
        r = subprocess.run(["java"] + JVMOPTS + ["-cp", scalac_cp, "dotty.tools.dotc.Main", "-classpath", scalac_cp,
                            "-color:never", "-d", classes, src], capture_output=True, text=True, timeout=600)
        if r.returncode != 0:
            sys.exit("scalac of InitDump.scala failed:\n" + (r.stdout + r.stderr)[-4000:])
        with open(done, "w") as fh:
            fh.write(SCALAC + "\n")
    return classes, scalac_cp


def library():
    return cs_fetch([f"org.scala-lang:scala-library:{SCALAC}"], classpath=True, intransitive=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--option", action="append", default=[])
    ap.add_argument("files", nargs="+")
    a = ap.parse_args()
    classes, scalac_cp = compiled()
    lib = library()
    out = os.path.abspath(a.out)
    products = os.path.join(out, "classes")
    os.makedirs(products, exist_ok=True)
    files = [os.path.abspath(f) for f in a.files]
    cmd = (["java"] + JVMOPTS + ["-cp", classes + ":" + scalac_cp, "dotty.tools.dotc.transform.init.InitDump",
            "-classpath", lib, "-color:never", "-Wsafe-init", "-Ysafe-init-global", "-d", products] + a.option + files)
    r = subprocess.run(cmd, cwd=os.path.dirname(files[0]), capture_output=True, text=True, timeout=600)
    phases, cur = {}, None
    for l in r.stdout.splitlines(keepends=True):
        if l.startswith("phase "):
            cur = l.split()[1]
            phases.setdefault(cur, [])
        elif cur is not None:
            phases[cur].append(l)
    for name in ("pickler", "initChecker"):
        with open(os.path.join(out, f"{name}.dump"), "w", encoding="utf-8") as fh:
            fh.write("".join(phases.get(name, [])))
    with open(os.path.join(out, "scalac.txt"), "w", encoding="utf-8") as fh:
        fh.write(r.stderr)
    with open(os.path.join(out, "exit"), "w") as fh:
        fh.write(f"{r.returncode}\n")
    return r.returncode


if __name__ == "__main__":
    sys.exit(main())
