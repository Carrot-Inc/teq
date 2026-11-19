"""tail_equality.py: two binaries' diagnostics, where the newer prints no location for a missing given's local callee.
Every line of the newer output must equal the older's, except an E172 headline ("No given instance of type .. was
found for <parameter> of <method|given instance> NAME in <owner>") whose callee NAME is a local definition: there the
newer line is the older line without its ` in <owner>` tail, and nothing else of it differs. A local definition is
read off teq's own definitions (`compiler check --dump-defs`): a definition of NAME whose owner chain holds a term (a
`val:`, `var:`, `def:` or `given:` link), or that stands in its class's parent argument (a parenthesis opened after
the class's name and still open at the definition), and whose innermost owner's span holds the diagnostic's place.

  tail_equality.py outputs <teq> <older dir> <newer dir>
      the errors suite's records of each program (tests/run_errors.sh --write: out/errors-base, out/errors-tests),
      paired by name; the local check reads the program under tests/errors.
  tail_equality.py programs <older teq> <newer teq> <scala-library jar> <program dir or file>...
      each program run by both, `compiler check --no-cache --std scala-library --classpath <jar> <its files>`.
  tail_equality.py records <older record> <newer record>
      a project's record (errors.txt, deviations.json, run.json), item by item; a changed item is only reported,
      since its sources are the project's.

Prints, per input, the lines compared, the lines equal and the lines that lost a local's tail (each named), and every
other difference; exits 1 when there is one. A JSON analysis line's `"ms"` timings are read as one value, since they
change with every run."""
import json
import os
import re
import subprocess
import sys
import tempfile

HEAD = re.compile(r"^(?P<pre>.*No given instance of type .* was found for .* of (?:method|given instance) (?P<name>(?:(?! in )[^\n])*?[^\s.]))"
                  r"(?P<tail> in (?:class|object|trait) (?:(?! in )[^\n])*?[^\s.]| in package [\w.]+)$")
MS = re.compile(r'"ms":\{[^{}]*\}')
PLACE = re.compile(r"^(?P<file>[^\s:][^:]*\.scala):(?P<line>\d+):(?P<col>\d+): error: ")
TERM = ("val:", "var:", "def:", "given:")


def defs_of(teq, files):
    """teq's definitions of these files: {file basename: [(kind, name, chain, (start, end) of the definition)]}."""
    r = subprocess.run([teq, "compiler", "check", "--dump-defs"] + files, capture_output=True, text=True, timeout=300)
    out, cur = {}, None
    for l in r.stdout.splitlines():
        p = l.split("\t")
        if p[0] == "file":
            cur = os.path.basename(p[1])
            out.setdefault(cur, [])
        elif p[0] == "def" and len(p) >= 6 and cur and re.fullmatch(r"\d+:\d+", p[5]):
            s, e = (int(x) for x in p[5].split(":"))
            ns = int(p[4].split(":")[1]) if re.fullmatch(r"\d+:\d+", p[4]) else s
            out[cur].append((p[1], p[2], p[3], (s, e), ns))
    return out


def offset(path, line, col):
    text = open(path, encoding="utf-8").read().split("\n")
    return sum(len(x) + 1 for x in text[:line - 1]) + col - 1


def is_local(defs, sources, line_text, name):
    """Whether NAME at this diagnostic's place is a local definition of its file."""
    m = PLACE.match(line_text)
    if not m:
        return False
    base = os.path.basename(m.group("file"))
    path = sources.get(base)
    if path is None:
        return False
    at = offset(path, int(m.group("line")), int(m.group("col")))
    ds = defs.get(base, [])
    text = open(path, encoding="utf-8").read()
    for kind, n, chain, (ds_, de), _ in ds:
        if n != name or "/" not in chain:
            continue
        # the innermost owner: the definition named by the chain's last link, whose span holds the place
        last = chain.split("/")[-1]
        okind, oname = last.split(":", 1)
        for k2, n2, c2, (s, e), name_end in ds:
            if not (n2 == oname and k2 == okind and chain == f"{c2}/{last}" and s <= at <= e):
                continue
            if any(link.startswith(TERM) for link in chain.split("/")):
                return True
            # a class's parent argument: a parenthesis opened after the class's name and still open at the definition
            # (dotty types it in the class's local dummy's context, a term)
            between = text[name_end:ds_]
            if between.count("(") > between.count(")"):
                return True
    return False


def compare(label, old, new, defs=None, sources=None):
    """Line by line; returns the count of differences that are not a local's tail."""
    bad, tails, same = [], [], 0
    if len(old) != len(new):
        bad.append(f"{len(old)} lines, {len(new)} lines")
    for o, n in zip(old, new):
        if o == n or MS.sub('"ms":{}', o) == MS.sub('"ms":{}', n):
            same += 1
            continue
        m = HEAD.match(o)
        if m and n == m.group("pre"):
            if defs is None or is_local(defs, sources, o, m.group("name")):
                tails.append(n)
                continue
            bad.append(f"the tail of a callee that is not local: {o}")
            continue
        bad.append(f"- {o}\n    + {n}")
    print(f"{label}: {len(old)} lines, {same} equal, {len(tails)} a local's tail{', ' + str(len(bad)) + ' other' if bad else ''}")
    for t in tails:
        print(f"    tail: {t}")
    for b in bad:
        print(f"    OTHER {b}")
    return len(bad)


def program_sources(path):
    files = [path] if os.path.isfile(path) else sorted(os.path.join(path, f) for f in os.listdir(path) if f.endswith((".scala", ".sc")))
    return files, {os.path.basename(f): f for f in files}


def main(argv):
    bad = 0
    if argv[0] == "outputs":
        teq, od, nd = argv[1:4]
        for name in sorted(os.listdir(od)):
            if not name.endswith(".out") or not os.path.exists(os.path.join(nd, name)):
                continue
            prog = name[:-len(".out")]
            src = next((p for p in (f"tests/errors/{prog}.scala", f"tests/errors/{prog}") if os.path.exists(p)), None)
            old = open(os.path.join(od, name), encoding="utf-8", errors="replace").read().split("\n")
            new = open(os.path.join(nd, name), encoding="utf-8", errors="replace").read().split("\n")
            if old == new:
                continue
            files, sources = program_sources(src) if src else ([], {})
            bad += compare(prog, old, new, defs_of(teq, files) if files else {}, sources)
        print("outputs: every other program's output equal")
    elif argv[0] == "programs":
        ot, nt, jar = argv[1:4]
        for p in argv[4:]:
            files, sources = program_sources(p)
            outs = []
            for t in (ot, nt):
                with tempfile.TemporaryDirectory() as cache:
                    r = subprocess.run([t, "compiler", "check", "--no-cache", "--std", "scala-library", "--classpath", jar] + files,
                                       capture_output=True, text=True, timeout=300, env={**os.environ, "TEQ_CACHE_DIR": cache})
                outs.append((r.stdout + r.stderr).split("\n") + [f"exit {r.returncode}"])
            bad += compare(os.path.basename(p.rstrip("/")), outs[0], outs[1], defs_of(nt, files), sources)
    elif argv[0] == "records":
        def items(path):
            if path.endswith(".json"):
                d = json.load(open(path, encoding="utf-8"))
                return [x if isinstance(x, str) else json.dumps(x) for x in d.get("deviations", d.get("violations", []))]
            return [l.rstrip("\n").replace("\\n", "\n").split("\n")[0] for l in open(path, encoding="utf-8") if l.startswith("error ")]
        o, n = items(argv[1]), items(argv[2])
        heads = lambda xs: [x.split("\n")[0] for x in xs]
        bad += compare(os.path.basename(argv[2]), heads(o), heads(n))
    else:
        sys.exit(__doc__)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main(sys.argv[1:])
