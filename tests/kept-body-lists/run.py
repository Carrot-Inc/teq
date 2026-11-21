#!/usr/bin/env python3
"""The kept body's lists over lean products (src/typer/loader/bodies.rs, `filled_clause`): a library's kept body that
applies a USER-DEFINED callee's lists (a using or an implicit clause written whole, short, named or searched; an
overloaded alternative; an extractor's implicit arguments; a constructor's clauses as the reference normalizes them)
built over the library's lean products must print what scalac's build prints. gen.py makes the programs (its docstring
names every axis), record.py wrote expected.tsv from scalac 3.8.4 alone; each program is built over A's lean products
on two legs (js: lean JavaScript on node; interp: the interpreter) and each leg's observable (the run's output, or a
refusal) is held to scalac's: its output exactly, or, where scalac refuses the program, a refusal. A test is
`<program>/<leg>`; the ones expected to differ from scalac are tests/pending/kept-body-lists.txt
(tests/support/pending.py, whose --update and --add this passes on).

tests/kept-body-cut holds the other side: the lean std's own callees, which take master's reading.
mutants/ holds the controls: one patch per answer filled_clause gives, each of which this suite must fail on
(mutants/controls.sh builds and runs them).

  run.py [--update] [--add]

It fails, with a line each, where a generated program has no expected cell or a cell no program, and where a command
timed out (a timed-out leg is a failure no list holds). TEQ names the binary (target/release/teq), NODE node (node),
JOBS the programs at once (default 6)."""
import os
import re
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
TEQ = os.environ.get("TEQ", os.path.join(REPO, "target/release/teq"))
NODE = os.environ.get("NODE", "node")
JOBS = int(os.environ.get("JOBS", "6"))
LEGS = ("js", "interp")
EXPECTED = os.path.join(HERE, "expected.tsv")
SUITE = "kept-body-lists"


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
    """The two legs' observables of one program: ("out", text), ("refused", first line) or ("timeout", "")."""
    a, b = os.path.join(row, "A.scala"), os.path.join(row, "B.scala")
    ap = os.path.join(work, "ap")
    rc, out, err = run([TEQ, "compiler", "check", "--products", ap, "--sourceroot", row, a], work)
    if rc is None:
        return {l: ("timeout", "A's products") for l in LEGS}
    if rc != 0:
        return {l: ("refused", "A's products: " + first_line(err + out, work)) for l in LEGS}
    res = {}
    js = os.path.join(work, "o.js")
    rc, out, err = run([TEQ, "compiler", "build", "--sourceroot", row, "--classpath", ap, b, "-o", js], work)
    if rc is None:
        res["js"] = ("timeout", "build")
    elif rc != 0:
        res["js"] = ("refused", first_line(err + out, work))
    else:
        rc, out, err = run([NODE, js], work, timeout=60)
        res["js"] = ("timeout", "node") if rc is None else (("out", out) if rc == 0 else ("fails", first_line(err, work)))
    rc, out, err = run([TEQ, "interp", "--sourceroot", row, "--classpath", ap, b], work, env={"LC_ALL": "C"})
    if rc is None:
        res["interp"] = ("timeout", "interp")
    elif rc != 0:
        res["interp"] = ("refused" if "error" in err else "fails", first_line(err + out, work))
    else:
        res["interp"] = ("out", out)
    return res


def one(args):
    name, rows = args
    with tempfile.TemporaryDirectory(prefix="kb-lists-") as work:
        return name, legs(os.path.join(rows, name), work)


def cell(text):
    # an object's identity (`@41c67ea6`) and an anonymous class's generated suffix are no observable of the lists
    text = re.sub(r"\$anon\$[0-9a-z_]+", "$anon", re.sub(r"@[0-9a-f]+\b", "@<identity>", text))
    return text.rstrip("\n").replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n")


def main(argv):
    expected = {}
    for line in open(EXPECTED, encoding="utf-8"):
        if line.startswith("#") or not line.strip():
            continue
        n, v = line.rstrip("\n").split("\t", 2)[:2]
        expected[n] = v
    with tempfile.TemporaryDirectory(prefix="kb-lists-gen-") as gen:
        subprocess.run([sys.executable, os.path.join(HERE, "gen.py"), gen], stdout=subprocess.DEVNULL, check=True)
        names = open(os.path.join(gen, "rows.txt")).read().split()
        with ThreadPoolExecutor(JOBS) as pool:
            got = dict(pool.map(one, [(n, os.path.join(gen, "rows")) for n in names]))
    bad, results, held = [], [], 0
    for n in names:
        want = expected.pop(n, None)
        if want is None:
            bad.append(f"{n}: no expected cell (record.py after a change of gen.py)")
            continue
        for l in LEGS:
            kind, text = got[n][l]
            if kind == "timeout":
                bad.append(f"{n} {l}: a command timed out ({text})")
                continue
            held += 1
            if want == "REFUSED":
                ok, diff = kind == "refused", (f"prints {cell(text)[:100]} where scalac refuses" if kind == "out" else f"{kind}: {text[:100]}")
            else:
                ok = kind == "out" and cell(text) == want
                diff = f"prints {cell(text)[:100]} where scalac prints {want[:100]}" if kind == "out" else f"{kind}: {text[:100]} where scalac prints {want[:100]}"
            results.append(f"pass {n}/{l}\n" if ok else f"fail {n}/{l}\t{diff}\n")
    bad += [f"{n}: an expected cell no program has" for n in expected]
    for b in bad[:40]:
        print("FAIL", b)
    print(f"{SUITE}: {len(names)} programs, {held} cells held to scalac's ({sum(r.startswith('pass') for r in results)} as scalac's)")
    with tempfile.NamedTemporaryFile("w", prefix="kb-lists-results-", suffix=".txt", delete=False) as fh:
        fh.write("".join(results))
        res = fh.name
    if os.environ.get("KB_LISTS_RESULTS"):
        # every cell's verdict for mutants/controls.sh, a timed-out leg as `timeout`
        with open(os.environ["KB_LISTS_RESULTS"], "w") as fh:
            fh.write("".join(results) + "".join(f"timeout {b.split(':')[0].replace(' ', '/')}\n" for b in bad if "timed out" in b))
    try:
        flags = [f for f in argv if f in ("--update", "--add")]
        rc = subprocess.run([sys.executable, os.path.join(REPO, "tests", "support", "pending.py"), SUITE, res] + flags).returncode
    finally:
        os.unlink(res)
    return 1 if bad or rc != 0 else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
