#!/usr/bin/env python3
"""equality.py <repo> <teq base> <teq head> <out> <workers> <base's number> <head's number>: every product of a build
of each tests/cases/*.scala and tests/tasty/src/*.scala, `compiler check --products` and `compiler build --target jvm
--products` (scala-library 3.8.4 on the class path, fetched by $COURSIER, `cs` by default), by both binaries from the
repository root; the directories byte for byte but teq-products.json, which is compared without `abi` and `identity`,
each side's `abi` checked to be its binary's number. Writes <out>/equality.tsv (a row per input and mode: its verdict
and detail) and the trees <out>/master/<mode>/<name> and <out>/head/<mode>/<name>, each build's output in
<out>/logs/<side>/<mode>/<name>.log; right after each build, the manifest of what it wrote (<out>/sums/<side>/<mode>/
<name>.sum: its exit, its log's and each file's bytes and digest, corpus_judge.py's `seal_unit`), and after the table
its sum (<out>/equality.sum), which tests/corpus-bytes/corpus_judge.py holds the trees, the logs and the table to: a
build's exit is judged there by a whitelist, 0 or 1 with teq's error line in its log. <out> must be new or empty: a file an earlier run left there would be sealed as this run's."""
import concurrent.futures
import glob
import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import corpus_judge  # noqa: E402

repo, master, head, out, workers = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5])
abi_base, abi_head = int(sys.argv[6]), int(sys.argv[7])
if os.path.isdir(out) and os.listdir(out):
    sys.exit(f"equality: {out} is not empty")
os.makedirs(out, exist_ok=True)
fetched = subprocess.run([os.environ.get("COURSIER", "cs"), "fetch", "--classpath", "--intransitive",
                          "org.scala-lang:scala-library:3.8.4"], capture_output=True, text=True)
SL = (fetched.stdout.strip().splitlines() or [""])[-1]
if fetched.returncode != 0 or not SL:
    sys.exit("equality: scala-library 3.8.4 not fetched")
inputs = sorted(glob.glob(f"{repo}/tests/cases/*.scala")) + sorted(glob.glob(f"{repo}/tests/tasty/src/*.scala"))


def run(binary, mode, src, d, log):
    """The build's exit (`timeout` past 120 s, a signal's as 128 and its number), its output in `log`."""
    rel = os.path.relpath(src, repo)
    args = [binary, "compiler", "check", "--products", d, rel] if mode == "check" else [binary, "compiler", "build", "--target", "jvm", "--products", d, "--classpath", SL, rel]
    os.makedirs(os.path.dirname(log), exist_ok=True)
    with open(log, "wb") as fh:
        try:
            code = subprocess.run(args, cwd=repo, stdout=fh, stderr=subprocess.STDOUT, timeout=120).returncode
        except subprocess.TimeoutExpired:
            return "timeout"
    return 128 - code if code < 0 else code


def files(d):
    res = {}
    for base, _, names in os.walk(d):
        for n in names:
            p = os.path.join(base, n)
            res[os.path.relpath(p, d)] = open(p, "rb").read()
    return res


def strip(text):
    m = json.loads(text)
    m.pop("identity", None)
    abi = m.pop("abi", None)
    return m, abi


def one(job):
    mode, src = job
    name = corpus_judge.unit_name(src)
    dm, dh = f"{out}/master/{mode}/{name}", f"{out}/head/{mode}/{name}"
    cm = run(master, mode, src, dm, corpus_judge.log_path(out, "master", mode, name))
    corpus_judge.seal_unit(out, "master", mode, name, cm)
    ch = run(head, mode, src, dh, corpus_judge.log_path(out, "head", mode, name))
    corpus_judge.seal_unit(out, "head", mode, name, ch)
    if cm == "timeout" or ch == "timeout":
        return mode, name, "timeout", f"{cm} {ch}"
    if cm != ch:
        return mode, name, "exit-differs", f"{cm} {ch}"
    fm, fh = files(dm), files(dh)
    if not fm and not fh:
        return mode, name, "both-wrote-nothing", str(cm)
    diff = sorted(set(fm) ^ set(fh)) + sorted(p for p in set(fm) & set(fh) if p != "teq-products.json" and fm[p] != fh[p])
    if "teq-products.json" in fm and "teq-products.json" in fh:
        (a, abi_m), (b, abi_h) = strip(fm["teq-products.json"]), strip(fh["teq-products.json"])
        if a != b:
            diff.append("teq-products.json(rest)")
        if (abi_m or 0) != abi_base or (abi_h or 0) != abi_head:
            diff.append(f"abi base={abi_m} head={abi_h}")
    if diff:
        return mode, name, "differs", " ".join(diff[:5])
    return mode, name, "equal", f"{len(fm)} files exit {cm}"


jobs = [(m, s) for s in inputs for m in ("check", "jvm")]
counts = {}
files_equal = 0
table = os.path.join(out, "equality.tsv")
with open(table, "w") as rows, concurrent.futures.ThreadPoolExecutor(workers) as pool:
    for mode, name, verdict, detail in pool.map(one, jobs):
        counts[(mode, verdict)] = counts.get((mode, verdict), 0) + 1
        if verdict == "equal":
            files_equal += int(detail.split()[0])
        rows.write(f"{mode}\t{name}\t{verdict}\t{detail}\n")
corpus_judge.seal_table(table)
for k in sorted(counts):
    print(k[0], k[1], counts[k])
print("files compared equal:", files_equal)
