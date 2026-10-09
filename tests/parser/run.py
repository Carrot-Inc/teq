#!/usr/bin/env python3
"""The mutation suite of the parser's recovery.

  run.py [--teq BIN] [--jobs N] [--flags F]   teq over the corpus: out/parser/teq.jsonl and the table
  run.py --scalac                             scalac 3.8.4 over the corpus: out/parser/scalac.jsonl
  run.py --table                              the table of both: out/parser/teq.jsonl, the frozen scalac
  run.py --sites                              the definitions lost per mutation by the recovery site
  run.py --freeze LABEL                       writes tests/parser/baseline/ from the last runs of both
  run.py --gate                               teq over the corpus, checked against tests/parser/expect.tsv
                                              and tests/parser/exceptions.tsv
  run.py --expect                             writes tests/parser/expect.tsv from the last teq run
  run.py --fixtures                           the complete-list fixtures, against their .teq and .scalac
  run.py --fixtures --record-teq|--record-scalac   writes the fixtures' .teq or .scalac lists

The corpus is tests/parser/mutate.py's, written to out/parser/mutants and refused unless its digest is
tests/parser/digest. Per mutant, teq's recovered tree (`teq compiler check --dump-defs`) is compared with the
tree of the file it was made from: a definition of the original is kept when the mutant has one of
the same kind and name with the same owner at the same place (shifted past the edit); it is
swallowed when none has its kind, name and place, and has a wrong owner when one does under
another owner. The definitions the edit touches and those nested in one whose name it touches are
left out. Then the mutant is checked with the flags F (`teq compiler check`), each run bounded by TIMEOUT
seconds: a run over it is a hang, an exit other than 0 and 1 a crash. Its errors, the first
syntax error included, are its diagnostics; the cascade is the errors past the first.

The gate fails on any hang, crash, swallowed definition, wrong owner or moved span, mutant by
mutant, but for the losses tests/parser/exceptions.tsv records for a mutant with their reason (a
mutant the edit made a valid program of, or one whose definition's keyword it deleted), which
fail when they no longer occur; and on a mutation or class whose errors or cascade rose above
tests/parser/expect.tsv's.

The fixtures (tests/parser/fixtures/, a file or a directory of files each) hold the
representative shapes with their complete diagnostic lists: `<name>.teq` as teq reports them,
which the run must give exactly, and `<name>.scalac` as scalac 3.8.4 reports them. Both are
normalised (`normalise`): a diagnostic is its file, its 1-based line and column, its severity
and its message's first line, teq's `expected X, found Y` read as scalac's `X expected, but Y
found` with scalac's token names mapped to teq's, a type mismatch and a missing name reduced
to their kind and name. The lists are compared without the columns (teq marks a span's start,
scalac a tree's point), and every difference is a departure that
tests/parser/fixtures/departures.tsv lists with its reason.
"""
import concurrent.futures
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
import mutate  # noqa: E402

OUT = os.path.join(ROOT, "out", "parser")
MUTANTS = os.path.join(OUT, "mutants")
TIMEOUT = 20
DIAG = re.compile(r"^(?P<path>[^:\n]+):(?P<line>\d+):(?P<col>\d+): (?P<severity>error|warning): (?P<message>.*)$")


def arg(name, default=None):
    if name in sys.argv:
        return sys.argv[sys.argv.index(name) + 1]
    return default


def corpus():
    ms = mutate.mutants()
    digest = mutate.digest(ms)
    frozen = open(os.path.join(HERE, "digest")).read().strip()
    if digest != frozen:
        sys.exit(f"the corpus's digest is {digest}, not tests/parser/digest's {frozen}: the generator, the manifest or a case changed")
    os.makedirs(MUTANTS, exist_ok=True)
    for m in ms:
        path = os.path.join(MUTANTS, m["id"] + ".scala")
        if not os.path.exists(path) or open(path, encoding="utf-8").read() != m["mutated"]:
            with open(path, "w", encoding="utf-8") as f:
                f.write(m["mutated"])
    return ms


def run(cmd):
    try:
        p = subprocess.run(cmd, cwd=ROOT, capture_output=True, timeout=TIMEOUT)
        return p.returncode, p.stdout.decode("utf-8", "replace"), p.stderr.decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        return "timeout", "", ""


def parse_dump(text):
    defs, skips, errors = [], [], []
    for line in text.splitlines():
        f = line.split("\t")
        if f[0] == "def" and len(f) in (6, 7):
            s, e = f[4].split(":")
            r = None if f[5] == "-" else tuple(int(x) for x in f[5].split(":"))
            incomplete = len(f) == 7 and f[6] == "incomplete"
            defs.append({"kind": f[1], "name": f[2], "owner": f[3], "span": (int(s), int(e)), "range": r, "incomplete": incomplete})
        elif f[0] == "skip":
            s, e = f[2].split(":")
            skips.append({"site": f[1], "span": (int(s), int(e))})
        elif f[0] == "error":
            s, e = f[1].split(":")
            errors.append({"span": (int(s), int(e)), "message": f[2] if len(f) > 2 else ""})
    return defs, skips, errors


def fresh(name):
    return re.search(r"\$\d+$", name) is not None


class Edit:
    """The edit of a mutant in bytes of the UTF-8 text, from the mutate.py offsets in characters."""

    def __init__(self, src, m):
        self.start = len(src[:m["start"]].encode())
        self.end = len(src[:m["end"]].encode())
        self.delta = len(m["text"].encode()) - (self.end - self.start)

    def touches(self, s, e):
        return s <= self.end and e >= self.start

    def shift(self, s, e):
        if e <= self.start:
            return (s, e)
        return (s + self.delta, e + self.delta)


def compare(orig, mutant, edit, at_site_owner_prefixes=True):
    """kept, swallowed, wrong owner, moved, with the names of what was lost."""
    excluded_owners = []
    for d in orig:
        if edit.touches(*d["span"]):
            excluded_owners.append(f"{d['owner']}/{d['kind']}:{d['name']}")
    by_place = {}
    for d in mutant:
        by_place.setdefault((d["kind"], d["name"], d["span"]), []).append(d)
    result = {"kept": 0, "swallowed": [], "wrong_owner": [], "moved": [], "at_site": 0}
    for d in orig:
        if fresh(d["name"]):
            continue
        if edit.touches(*d["span"]) or any(d["owner"] == o or d["owner"].startswith(o + "/") for o in excluded_owners):
            result["at_site"] += 1
            continue
        place = (d["kind"], d["name"], edit.shift(*d["span"]))
        found = by_place.get(place, [])
        label = f"{d['owner']}/{d['kind']}:{d['name']}@{d['span'][0]}"
        if not found:
            result["swallowed"].append(label)
            continue
        same = [x for x in found if x["owner"] == d["owner"]]
        if not same:
            result["wrong_owner"].append(label + " -> " + found[0]["owner"])
            continue
        r = d["range"]
        if r is not None and not edit.touches(*r) and same[0]["range"] != edit.shift(*r):
            result["moved"].append(label)
            continue
        result["kept"] += 1
    return result


def diagnostics(stderr):
    out = []
    for line in stderr.splitlines():
        m = DIAG.match(line)
        if m:
            out.append({"severity": m["severity"], "line": int(m["line"]), "col": int(m["col"]), "message": m["message"]})
    return out


def teq_mutant(teq, flags, m, orig_dump):
    src = open(os.path.join(ROOT, m["file"]), encoding="utf-8").read()
    path = os.path.relpath(os.path.join(MUTANTS, m["id"] + ".scala"), ROOT)
    code, out, _ = run([teq, "compiler", "check", "--dump-defs", path])
    record = {"id": m["id"], "operator": m["operator"], "class": m["class"]}
    if code == "timeout" or code not in (0,):
        record["dump"] = "hang" if code == "timeout" else f"crash {code}"
        defs, skips, perrors = [], [], []
    else:
        defs, skips, perrors = parse_dump(out)
    record["parse_errors"] = len(perrors)
    record["skips"] = [f"{s['site']}:{s['span'][0]}:{s['span'][1]}" for s in skips]
    record["structure"] = compare(orig_dump, defs, Edit(src, m))
    cmd = [teq, "compiler", "check"] + flags + (m["teq_flags"].split() if m["teq_flags"] else []) + [path]
    code, _, err = run(cmd)
    record["exit"] = code
    record["diagnostics"] = diagnostics(err)
    return record


def run_teq(teq, flags, jobs):
    ms = corpus()
    originals = sorted({m["file"] for m in ms})
    orig = {}

    def dump_original(path):
        code, out, _ = run([teq, "compiler", "check", "--dump-defs", path])
        return path, parse_dump(out)[0] if code == 0 else []

    with concurrent.futures.ThreadPoolExecutor(jobs) as pool:
        for path, defs in pool.map(dump_original, originals):
            orig[path] = defs
        records = list(pool.map(lambda m: teq_mutant(teq, flags, m, orig[m["file"]]), ms))
    with open(os.path.join(OUT, "teq.jsonl"), "w") as f:
        for r in records:
            f.write(json.dumps(r) + "\n")
    return records


def run_scalac():
    ms = corpus()
    lst = os.path.join(OUT, "scalac-list.tsv")
    with open(lst, "w") as f:
        for path in sorted({m["source"] for m in ms}):
            flags = next(m["scalac_flags"] for m in ms if m["source"] == path)
            f.write(f"o:{path}\t{os.path.join(ROOT, mutate.frozen(path))}\t{flags}\n")
        for m in ms:
            f.write(f"{m['id']}\t{os.path.join(MUTANTS, m['id'] + '.scala')}\t{m['scalac_flags']}\n")
    driver = os.path.join(HERE, "scalac", "Baseline.scala")
    with open(os.path.join(OUT, "scalac.jsonl"), "w") as out:
        p = subprocess.run(["scala-cli", "run", "-S", "3.8.4", "--jvm", "system", "--server=false", driver, "--", lst],
                           cwd=OUT, stdout=out, stderr=subprocess.PIPE)
    if p.returncode != 0:
        sys.exit(p.stderr.decode()[-2000:])


def scalac_records():
    ms = {m["id"]: m for m in mutate.mutants()}
    by_id = {}
    for line in open(os.path.join(OUT, "scalac.jsonl")):
        r = json.loads(line)
        by_id[r["id"]] = r
    records = []
    for mid, m in ms.items():
        r = by_id.get(mid)
        if r is None:
            continue
        src = open(os.path.join(ROOT, m["file"]), encoding="utf-8").read()
        orig = by_id.get("o:" + m["source"], {"defs": []})["defs"]

        def as_defs(ds, text):
            out = []
            for d in ds:
                at = len(text[:d["at"]].encode())
                out.append({"kind": d["kind"], "name": d["name"], "owner": d["owner"], "span": (at, at + len(d["name"].encode())), "range": None})
            return out

        diags = [d for d in r["diagnostics"] if d["line"] > 0]
        records.append({
            "id": mid, "operator": m["operator"], "class": m["class"],
            "structure": compare(as_defs(orig, src), as_defs(r["defs"], m["mutated"]), Edit(src, m)),
            "diagnostics": diags,
        })
    return records


def summary(records):
    rows = {}
    for r in records:
        for key in (r["operator"], "class:" + r["class"], "all"):
            row = rows.setdefault(key, {"mutants": 0, "hangs": 0, "crashes": 0, "swallowed": 0, "wrong_owner": 0, "moved": 0,
                                        "lossy": 0, "errors": 0, "cascade": 0})
            row["mutants"] += 1
            if r.get("dump") == "hang" or r.get("exit") == "timeout":
                row["hangs"] += 1
            elif str(r.get("dump", "")).startswith("crash") or r.get("exit") not in (None, 0, 1):
                row["crashes"] += 1
            s = {k: (len(v) if isinstance(v, list) else v) for k, v in r["structure"].items()}
            row["swallowed"] += s["swallowed"]
            row["wrong_owner"] += s["wrong_owner"]
            row["moved"] += s["moved"]
            row["lossy"] += 1 if s["swallowed"] or s["wrong_owner"] or s["moved"] else 0
            errors = sum(1 for d in r["diagnostics"] if d["severity"] == "error")
            row["errors"] += errors
            row["cascade"] += max(0, errors - 1)
    return rows


def table(teq, scalac):
    t, s = summary(teq), summary(scalac) if scalac else {}
    keys = [k for k, _, _ in mutate.VOCABULARY if k in t] + [k for k in ("class:syntactic", "class:lexical", "class:valid", "all") if k in t]
    head = "| mutation | mutants | hangs | crashes | swallowed (scalac) | wrong owner (scalac) | moved | lossy mutants (scalac) | errors (scalac) | cascade (scalac) |"
    lines = [head, "|" + "---|" * 10]
    for k in keys:
        a, b = t[k], s.get(k, {})
        g = lambda key: f"{a[key]} ({b[key]})" if b else f"{a[key]}"  # noqa: E731
        lines.append(f"| {k} | {a['mutants']} | {a['hangs']} | {a['crashes']} | {g('swallowed')} | {g('wrong_owner')} | {a['moved']} | {g('lossy')} | {g('errors')} | {g('cascade')} |")
    return "\n".join(lines)


def sites(records):
    """Per mutation, the definitions lost (swallowed or under a wrong owner) by the recovery that
    skipped them, `absorbed` where no recovery skipped the place (the lexer's suppressed layout
    inside an unclosed delimiter, or a construct that parsed it as its own)."""
    out = {}
    for r in records:
        skips = [(x.split(":")[0], int(x.split(":")[1]), int(x.split(":")[2])) for x in r.get("skips", [])]
        s = r["structure"]
        for lost in s["swallowed"] + s["wrong_owner"]:
            at = int(lost.split(" -> ")[0].rsplit("@", 1)[1])
            site = next((k for k, a, b in skips if a <= at <= b), "absorbed")
            row = out.setdefault(r["operator"], {})
            row[site] = row.get(site, 0) + 1
    lines = ["| mutation | lost definitions by the recovery that skipped them |", "|---|---|"]
    for k, _, _ in mutate.VOCABULARY:
        if k in out:
            cell = ", ".join(f"{site} {n}" for site, n in sorted(out[k].items(), key=lambda x: -x[1]))
            lines.append(f"| {k} | {cell} |")
    return "\n".join(lines)


def frozen_scalac():
    path = os.path.join(HERE, "baseline", "scalac.jsonl")
    return [json.loads(line) for line in open(path)] if os.path.exists(path) else None


def freeze(teq_records, label):
    """Writes the baseline: scalac's records from out/parser/scalac.jsonl, compacted, and teq's
    counts per mutant under `label`."""
    os.makedirs(os.path.join(HERE, "baseline"), exist_ok=True)
    with open(os.path.join(HERE, "baseline", "scalac.jsonl"), "w") as f:
        for r in scalac_records():
            structure = {k: (len(v) if isinstance(v, list) else v) for k, v in r["structure"].items()}
            r = dict(r, structure=structure, diagnostics=[[d["severity"][0], d["line"], d["col"], d["message"]] for d in r["diagnostics"]])
            f.write(json.dumps(r, separators=(",", ":")) + "\n")
    with open(os.path.join(HERE, "baseline", f"teq-{label}.tsv"), "w") as f:
        f.write("# id\toperator\tclass\texit\terrors\tswallowed\twrong owner\tmoved\n")
        for r in teq_records:
            s = r["structure"]
            errors = sum(1 for d in r["diagnostics"] if d["severity"] == "error")
            f.write(f"{r['id']}\t{r['operator']}\t{r['class']}\t{r['exit']}\t{errors}\t{len(s['swallowed'])}\t{len(s['wrong_owner'])}\t{len(s['moved'])}\n")


def scalac_baseline():
    """scalac's records with the diagnostics as run.py reads them."""
    rs = frozen_scalac()
    if rs is None:
        return scalac_records() if load("scalac.jsonl") else None
    for r in rs:
        r["diagnostics"] = [{"severity": {"e": "error", "w": "warning"}.get(d[0], "info"), "line": d[1], "col": d[2], "message": d[3]} for d in r["diagnostics"]]
    return rs


def load(name):
    path = os.path.join(OUT, name)
    return [json.loads(line) for line in open(path)] if os.path.exists(path) else None


def exceptions():
    """The structural losses tests/parser/exceptions.tsv allows, by mutant: `<id>\t<losses>\t<reason>`."""
    allowed = {}
    for line in open(os.path.join(HERE, "exceptions.tsv")):
        if line.startswith("#") or not line.strip():
            continue
        mid, losses, _reason = line.rstrip("\n").split("\t")[:3]
        allowed[mid] = int(losses)
    return allowed


def gate(records):
    failures = []
    allowed = exceptions()
    seen = set()
    for r in records:
        s = r["structure"]
        problems = []
        if r.get("dump") == "hang" or r.get("exit") == "timeout":
            problems.append("hang")
        elif str(r.get("dump", "")).startswith("crash") or r.get("exit") not in (0, 1):
            problems.append(f"crash ({r.get('dump') or r.get('exit')})")
        losses = [f"swallowed {x}" for x in s["swallowed"]] + [f"wrong owner {x}" for x in s["wrong_owner"]] + [f"moved {x}" for x in s["moved"]]
        if r["id"] in allowed:
            seen.add(r["id"])
            if len(losses) != allowed[r["id"]]:
                problems.append(f"{len(losses)} losses where tests/parser/exceptions.tsv allows {allowed[r['id']]}: " + "; ".join(losses))
        else:
            problems += losses
        if problems:
            failures.append(f"{r['id']} {r['operator']}: " + "; ".join(problems))
    for mid in sorted(set(allowed) - seen):
        failures.append(f"{mid}: listed in tests/parser/exceptions.tsv but not in the corpus")
    expected = {}
    for line in open(os.path.join(HERE, "expect.tsv")):
        if line.startswith("#") or not line.strip():
            continue
        key, errors, cascade = line.split("\t")[:3]
        expected[key] = (int(errors), int(cascade))
    rows = summary(records)
    for key, (errors, cascade) in expected.items():
        if key in rows and rows[key]["errors"] > errors:
            failures.append(f"{key}: {rows[key]['errors']} errors, above the recorded {errors}")
        if key in rows and rows[key]["cascade"] > cascade:
            failures.append(f"{key}: a cascade of {rows[key]['cascade']}, above the recorded {cascade}")
    return failures


def write_expect(records):
    """tests/parser/expect.tsv: the errors and the cascade per mutation and class, with the
    baseline's (teq before the recovery, scalac 3.8.4) beside them."""
    rows = summary(records)
    base = {}
    for line in open(os.path.join(HERE, "baseline", "teq-371d81a8.tsv")):
        if line.startswith("#"):
            continue
        f = line.rstrip("\n").split("\t")
        for key in (f[1], "class:" + f[2], "all"):
            e = base.setdefault(key, [0, 0])
            e[0] += int(f[4])
            e[1] += max(0, int(f[4]) - 1)
    sc = summary(scalac_baseline())
    keys = [k for k, _, _ in mutate.VOCABULARY if k in rows] + [k for k in ("class:syntactic", "class:lexical", "class:valid", "all") if k in rows]
    with open(os.path.join(HERE, "expect.tsv"), "w") as f:
        f.write("# key\terrors\tcascade\tbaseline errors\tbaseline cascade\tscalac errors\tscalac cascade\n")
        for k in keys:
            b, c = base.get(k, [0, 0]), sc.get(k, {"errors": 0, "cascade": 0})
            f.write(f"{k}\t{rows[k]['errors']}\t{rows[k]['cascade']}\t{b[0]}\t{b[1]}\t{c['errors']}\t{c['cascade']}\n")


FIXTURES = os.path.join(HERE, "fixtures")
SCALAC_TOKENS = [("eof", "end of file"), ("end of statement", "new line"), ("unindent", "end of indented block"), ("indent", "indented block")]


def fixtures():
    """Each fixture's name and its files: a file of tests/parser/fixtures, or a directory's files."""
    out = []
    for name in sorted(os.listdir(FIXTURES)):
        path = os.path.join(FIXTURES, name)
        if os.path.isdir(path):
            files = sorted(os.path.join(path, f) for f in os.listdir(path) if f.endswith(".scala"))
            out.append((name, files))
        elif name.endswith(".scala"):
            out.append((name[:-len(".scala")], [path]))
    return out


def normalise(file, line, col, severity, message):
    # scalac ends a message's first line with a space where more lines follow.
    message = message.rstrip()
    m = re.match(r"^expected (.+), found (.+)$", message)
    if m:
        message = f"{m[1]} expected, but {m[2]} found"
    m = re.match(r"^(.+) expected,? but (.+) found$", message)
    if m:
        found = m[2]
        for theirs, ours in SCALAC_TOKENS:
            if found == theirs or found == f"'{theirs}'":
                found = ours
        found = re.sub(r"^(string|integer|character|floating point|long) literal$", "literal", found)
        message = f"{m[1]} expected, but {found} found"
    if message.startswith("type mismatch:") or message.startswith("Found:"):
        message = "type mismatch"
    m = re.match(r"^(?:Not found|not found): (?:type )?(\S+)", message) or re.match(r"^type (\S+) not found", message)
    if m:
        message = f"not found: {m[1]}"
    return f"{os.path.basename(file)}:{line}:{col}: {severity}: {message}"


def teq_fixture(teq, files):
    """The exit of `teq compiler check` over the files and its diagnostics, normalised."""
    code, _, err = run([teq, "compiler", "check"] + files)
    out = []
    for line in err.splitlines():
        m = DIAG.match(line)
        if m:
            out.append(normalise(m["path"], int(m["line"]), int(m["col"]), m["severity"], m["message"]))
    return code, sorted(out, key=sort_key)


def sort_key(line):
    file, ln, col, rest = line.split(":", 3)
    return (file, int(ln), int(col), rest)


def without_column(line):
    file, ln, _col, rest = line.split(":", 3)
    return f"{file}:{ln}:{rest}"


def record_scalac_fixtures():
    lst = os.path.join(OUT, "fixtures-list.tsv")
    os.makedirs(OUT, exist_ok=True)
    fx = fixtures()
    with open(lst, "w") as f:
        for name, files in fx:
            f.write(f"{name}\t{' '.join(files)}\t\n")
    driver = os.path.join(HERE, "scalac", "Baseline.scala")
    p = subprocess.run(["scala-cli", "run", "-S", "3.8.4", "--jvm", "system", "--server=false", driver, "--", lst],
                       cwd=OUT, capture_output=True)
    if p.returncode != 0:
        sys.exit(p.stderr.decode()[-2000:])
    for line in p.stdout.decode().splitlines():
        r = json.loads(line)
        diags = [normalise(d["file"], d["line"], d["col"], d["severity"], d["message"]) for d in r["diagnostics"] if d["line"] > 0]
        with open(os.path.join(FIXTURES, r["id"] + ".scalac"), "w") as f:
            f.write("".join(d + "\n" for d in sorted(diags, key=sort_key)))


def departures():
    listed = {}
    path = os.path.join(FIXTURES, "departures.tsv")
    if os.path.exists(path):
        for line in open(path):
            if line.startswith("#") or not line.strip():
                continue
            name, side, diag, _reason = line.rstrip("\n").split("\t")[:4]
            listed.setdefault(name, set()).add((side, diag))
    return listed


def check_fixtures(teq, record_teq):
    failures = []
    listed = departures()
    for name, files in fixtures():
        code, got = teq_fixture(teq, files)
        # An exit other than 0 and 1 (a signal is a negative one) or the time running out is a
        # crash or a hang, whatever it printed before.
        if code not in (0, 1):
            failures.append(f"{name}: teq crashed or hung: exit {code}")
            continue
        teq_path = os.path.join(FIXTURES, name + ".teq")
        if record_teq:
            with open(teq_path, "w") as f:
                f.write("".join(d + "\n" for d in got))
        expected = open(teq_path).read().splitlines() if os.path.exists(teq_path) else None
        if expected != got:
            failures.append(f"{name}: teq reports\n    " + "\n    ".join(got) + "\n  where " + name + ".teq holds\n    " + "\n    ".join(expected or ["(nothing recorded)"]))
            continue
        scalac_path = os.path.join(FIXTURES, name + ".scalac")
        theirs = open(scalac_path).read().splitlines() if os.path.exists(scalac_path) else []
        ours, others = [without_column(d) for d in got], [without_column(d) for d in theirs]
        diff = {("teq", d) for d in ours if ours.count(d) > others.count(d)} | {("scalac", d) for d in others if others.count(d) > ours.count(d)}
        for side, d in sorted(diff - listed.get(name, set())):
            failures.append(f"{name}: a departure departures.tsv does not list: {side} only: {d}")
        for side, d in sorted(listed.get(name, set()) - diff):
            failures.append(f"{name}: departures.tsv lists what is no departure: {side} only: {d}")
    return failures


def main():
    jobs = int(arg("--jobs", str(os.cpu_count() or 4)))
    teq = arg("--teq", os.environ.get("TEQ", os.path.join(ROOT, "target", "release", "teq")))
    flags = arg("--flags", "").split()
    if "--scalac" in sys.argv:
        run_scalac()
        print(table(scalac_records(), None))
        return
    if "--sites" in sys.argv:
        print(sites(load("teq.jsonl")))
        return
    if "--table" in sys.argv:
        print(table(load("teq.jsonl"), scalac_baseline()))
        return
    if "--freeze" in sys.argv:
        freeze(load("teq.jsonl"), arg("--freeze"))
        return
    if "--expect" in sys.argv:
        write_expect(load("teq.jsonl"))
        return
    if "--fixtures" in sys.argv:
        if "--record-scalac" in sys.argv:
            record_scalac_fixtures()
            return
        failures = check_fixtures(teq, "--record-teq" in sys.argv)
        for f in failures:
            print("FAIL " + f)
        print("fixtures passed" if not failures else f"fixtures failed: {len(failures)}")
        sys.exit(1 if failures else 0)
    records = run_teq(teq, flags, jobs)
    if "--gate" in sys.argv:
        failures = gate(records)
        for f in failures[:200]:
            print("FAIL " + f)
        print(table(records, None))
        print("parser tests passed" if not failures else f"parser tests failed: {len(failures)}")
        sys.exit(1 if failures else 0)
    print(table(records, scalac_baseline()))


if __name__ == "__main__":
    main()
