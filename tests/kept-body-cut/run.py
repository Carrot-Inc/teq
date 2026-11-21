#!/usr/bin/env python3
"""The cut of the kept body's clause question (src/typer/loader/bodies.rs, `lean_std_callee`): a library's kept body
that calls a static or a constructor of the lean std's own source with a using or implicit clause takes master's
reading of its lists. This suite holds that cut's cells to what a user observes: defs.py reads the std's source for
every such definition, gen.py makes, for each it can call, a library A whose kept body calls it in the list forms
(the whole list written, named, short, none) and a program B printing the value; each is built over A's lean products
on two legs (js: lean JavaScript on node; interp: the interpreter), and the leg's observable (the run's output, or the
first line of the refusal of A's products or of B) is held to expected.tsv. The instrumented equality, each program's
conversion against master's byte for byte (it needs master's binary and a dump patch on both), is kept beside the
kept-body records, outside the tree.

  run.py              build and run every program on both legs, then check them against expected.tsv
  run.py --update     the same, then write expected.tsv from the run (each cell run twice: a cell whose two runs
                      differ is recorded as `varies` and not held)

The check fails, with a line each, where a generated program has no expected cell or a cell no program, where a
cell's observable is not the expected one, and where a command timed out. TEQ names the binary (target/release/teq),
NODE node (node), JOBS the programs at once (default 6)."""
import os, re, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
TEQ = os.environ.get("TEQ", os.path.join(REPO, "target/release/teq"))
NODE = os.environ.get("NODE", "node")
JOBS = int(os.environ.get("JOBS", "6"))
LEGS = ("js", "interp")
EXPECTED = os.path.join(HERE, "expected.tsv")


def first_line(text, work):
    """A refusal's or a failure's first line, with the work directory and the machine's paths taken out."""
    for line in text.splitlines():
        if line.strip():
            line = line.replace(work + "/", "").replace(work, "")
            return re.sub(r"(?:/[\w.\-]+)+/([\w.\-]+\.(?:scala|js))", r"\1", line).strip()
    return ""


def run(cmd, cwd, env=None, timeout=120):
    try:
        p = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout, env={**os.environ, "LC_ALL": "C.UTF-8", **(env or {})})
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        return None, "", ""


def legs(row, work):
    """The two legs' observables of one program."""
    a, b = os.path.join(row, "A.scala"), os.path.join(row, "B.scala")
    ap = os.path.join(work, "ap")
    rc, out, err = run([TEQ, "compiler", "check", "--products", ap, "--sourceroot", row, a], work)
    if rc is None:
        return {l: "TIMEOUT" for l in LEGS}
    if rc != 0:
        why = "A's products REFUSED: " + first_line(err + out, work)
        return {l: why for l in LEGS}
    res = {}
    js = os.path.join(work, "o.js")
    rc, out, err = run([TEQ, "compiler", "build", "--sourceroot", row, "--classpath", ap, b, "-o", js], work)
    if rc is None:
        res["js"] = "TIMEOUT"
    elif rc != 0:
        res["js"] = "REFUSED: " + first_line(err + out, work)
    else:
        rc, out, err = run([NODE, js], work, timeout=60)
        res["js"] = "TIMEOUT" if rc is None else (out if rc == 0 else "RUN FAILS: " + first_line(err, work))
    rc, out, err = run([TEQ, "interp", "--sourceroot", row, "--classpath", ap, b], work, env={"LC_ALL": "C"})
    if rc is None:
        res["interp"] = "TIMEOUT"
    elif rc != 0:
        res["interp"] = ("REFUSED: " if "error" in err else "RUN FAILS: ") + first_line(err + out, work)
    else:
        res["interp"] = out
    return res


def one(args):
    name, rows = args
    with tempfile.TemporaryDirectory(prefix="kb-cut-") as work:
        return name, legs(os.path.join(rows, name), work)


def cell(text):
    # an object's identity (`@41c67ea6`, `@1`) and an anonymous class's generated suffix are no observable of the cut
    text = re.sub(r"\$anon\$[0-9a-z_]+", "$anon", re.sub(r"@[0-9a-f]+\b", "@<identity>", text))
    return text.rstrip("\n").replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n")


def main(argv):
    update = "--update" in argv
    with tempfile.TemporaryDirectory(prefix="kb-cut-gen-") as gen:
        declared = os.path.join(gen, "declared.json")
        with open(os.path.join(gen, "defs.jsonl"), "w") as fh:
            subprocess.run([sys.executable, os.path.join(HERE, "defs.py"), os.path.join(REPO, "std"), declared], stdout=fh, check=True)
        subprocess.run([sys.executable, os.path.join(HERE, "gen.py"), os.path.join(gen, "defs.jsonl"), declared, gen], stdout=subprocess.DEVNULL, check=True)
        rows = os.path.join(gen, "rows")
        names = open(os.path.join(gen, "progs", "rows.txt")).read().split()
        with ThreadPoolExecutor(JOBS) as pool:
            got = dict(pool.map(one, [(n, rows) for n in names]))
            again = dict(pool.map(one, [(n, rows) for n in names])) if update else None
    bad = []
    if update:
        lines = ["# program\tleg\tobservable (the run's output, or the first line of a refusal; `varies` where two runs differ)\n"]
        for n in names:
            for l in LEGS:
                v = cell(got[n][l]) if got[n][l] == again[n][l] else "varies"
                if "TIMEOUT" in (got[n][l], again[n][l]):
                    bad.append(f"{n} {l}: a command timed out, nothing recorded")
                lines.append(f"{n}\t{l}\t{v}\n")
        if not bad:
            open(EXPECTED, "w").write("".join(lines))
    expected = {}
    for line in open(EXPECTED, encoding="utf-8"):
        if line.startswith("#") or not line.strip():
            continue
        n, l, v = line.rstrip("\n").split("\t", 2)
        expected[(n, l)] = v
    held = 0
    for n in names:
        for l in LEGS:
            want = expected.pop((n, l), None)
            if want is None:
                bad.append(f"{n} {l}: no expected cell (run.py --update after a change of the std's definitions)")
            elif got[n][l] == "TIMEOUT":
                bad.append(f"{n} {l}: a command timed out")
            elif want != "varies":
                held += 1
                if cell(got[n][l]) != want:
                    bad.append(f"{n} {l}: {cell(got[n][l])[:120]} where expected.tsv holds {want[:120]}")
    bad += [f"{n} {l}: an expected cell no program has" for n, l in expected]
    for b in bad[:40]:
        print("FAIL", b)
    print(f"kept-body-cut: {len(names)} programs, {held} cells held to expected.tsv" + (f", {len(bad)} failing" if bad else ", as recorded"))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
