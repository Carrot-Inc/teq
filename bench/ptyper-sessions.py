#!/usr/bin/env python3
"""A session's builds at a count of workers against one worker: `teq
watch` driven over stdin, its first build, its warm full rebuilds and its retypes told apart, the counts
interleaved session by session with the order alternating each round.

    python3 bench/ptyper-sessions.py --out <file.jsonl> --corpus frontend|frontend-undeclared|api|app-corpus
        --kind split|check|index|jvm [--counts 1,8] [--rounds 6] [--cycles 2] [--app <build.json>]
        [--app-root <dir>] [--corpus-dir <dir>] [--env K=V ...] <teq>
    python3 bench/ptyper-sessions.py --summary <file.jsonl>...

A session's request after its first build alternates: three retypes, each the largest program file of the
corpus handed in (`text`) with a comment line appended or taken off again, then a full rebuild
(`TEQ_COMPACT_EVERY=4`, the fourth request taking the full path whatever it brings); `--cycles` of those. The
first retype after a full build is told apart from the two after it. Nothing is written to the corpus: a
split or JVM session writes into a directory of its own under the system's temporary directory. A count of
one or any other count is `TEQ_SESSION_WORKERS=<n>`, every full build at n workers, unless `--threads` gives
the count as `--threads <n>`; `auto` gives none, the session's own rule. How each full build was typed is
read from `TEQ_SESSION_WORKERS_LOG`: a count above one that a full build did not attempt (`joined <n>`, or
`retried <n>` where the attempt of n gave way) ends the run, and the round's line names what the automatic
count chose.

`frontend` is the application's frontend as the gate's build file describes it (its cacheable state
declared), `frontend-undeclared` the same without `--cacheable-state`, `description` the export `--app`
names (an sbt-teq description, a JVM one's target kept, `--extra` arguments added: a compiler resident's
`--own` roots, `--all-mains`), `api` the application's API side as
bench/app/api-check.sh checks it, `app-corpus` bench/app/gen.py's corpus as tests/watch-memory.sh types it
(its frontend and shared trees, the class catalog declared).

Each line of the output: the corpus, the kind, the count, the round, the request (`first`, `full`, `retype1`,
`retype2`, `retype3`), the answer's `ms` and whether it was incremental, the wall time, the machine's load.
The summary: per corpus, kind and request, the minimum and median of the type phase per count and their
ratio to one worker's.
"""
import argparse, json, os, re, shutil, statistics, subprocess, sys, tempfile, time


def load():
    return os.getloadavg()[0]


def scala_files(cwd, inputs):
    out = []
    for i in inputs:
        p = os.path.join(cwd, i)
        if os.path.isfile(p):
            out.append(p)
        for d, ds, fs in os.walk(p):
            ds.sort()
            out.extend(os.path.join(d, f) for f in sorted(fs) if f.endswith(".scala"))
    return out


def corpus(args):
    """The session's directory, its inputs (before the flags) and its flags."""
    if args.corpus in ("frontend", "frontend-undeclared", "description"):
        if not args.app:
            sys.exit(f"the corpus {args.corpus} needs --app <build.json>")
        d = json.load(open(args.app))
        root = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(args.app)), d.get("root", "../../..")))
        flags = ["--classpath", ":".join(d["classpath"])]
        if d.get("maxInlines"):
            flags += ["--max-inlines", str(d["maxInlines"])]
        flags += ["--strict-equality"] if d.get("strictEquality") else []
        flags += ["--kind-projector"] if d.get("kindProjector") else []
        flags += ["--werror"] if d.get("werror") else []
        if args.corpus == "frontend":
            for n in d.get("cacheableState", []):
                flags += ["--cacheable-state", n]
        if d.get("target") == "jvm":
            flags += ["--target", "jvm", "--std=scala-library"]
        small = d.get("modulePerFile", [])
        split = ["--module-per-file", ",".join(small)] if small else []
        return root, d["sources"], flags + args.extra, split + ["--hot"]
    if args.corpus == "api":
        lists = [os.environ.get(v, "") for v in ("APP_MODULES", "APP_CLASSPATH")]
        if not args.app_root or not all(os.path.isfile(f) for f in lists) or "APP_FLAGS" not in os.environ:
            sys.exit("the corpus api needs --app-root (or APP_ROOT), APP_MODULES, APP_CLASSPATH and APP_FLAGS, as bench/app/api-check.sh")
        root = args.app_root
        sources = [w for l in open(lists[0]) if not l.startswith("#") for w in l.split()]
        cp = ":".join(l.strip().replace("~/", os.path.expanduser("~") + "/", 1) for l in open(lists[1]) if l.strip())
        flags = ["--classpath", cp, *os.environ["APP_FLAGS"].split(), "--std=scala-library", "--target", "jvm"]
        return root, sources, flags, []
    if args.corpus in ("app-corpus", "app-corpus-undeclared"):
        declared = ["--cacheable-state", "meridian.web.css.Catalog"] if args.corpus == "app-corpus" else []
        return args.corpus_dir, ["shared", "frontend"], ["--classpath", args.classpath, *declared], ["--module-per-file", "meridian.frontend", "--hot"]
    sys.exit(f"no corpus {args.corpus}")


def read_answer(proc, seconds):
    line = proc.stdout.readline()
    if not line:
        raise RuntimeError("the session ended: " + proc.stderr.read()[-400:])
    return json.loads(line)


def session(args, cwd, inputs, flags, split, count, rnd, out):
    outdir = tempfile.mkdtemp(prefix="teq-sessions-")
    kind = args.kind
    target = {"split": ["--split", os.path.join(outdir, "out")] + split, "check": ["--check"], "index": ["--check", "--index"], "jvm": ["-o", os.path.join(outdir, "out")]}[kind]
    if kind == "jvm" and "--target" not in flags:
        target = ["--target", "jvm", "--std=scala-library"] + target
    threads = ["--threads", str(count)] if args.threads and count != "auto" else []
    cmd = [args.teq, "compiler", "watch", *inputs, *flags, *target, *threads]
    env = {k: v for k, v in os.environ.items() if k not in ("TEQ_SESSION_WORKERS", "TEQ_SESSION_THREADS", "TEQ_THREADS", "TEQ_FORK", "TEQ_COMPACT_EVERY")}
    env["TEQ_COMPACT_EVERY"] = "4"
    log = os.path.join(outdir, "workers.log")
    env["TEQ_SESSION_WORKERS_LOG"] = log
    if count != "auto" and not args.threads:
        env["TEQ_SESSION_WORKERS"] = str(count)
    for kv in args.env:
        k, v = kv.split("=", 1)
        env[k] = v
    files = scala_files(cwd, inputs)
    edited = max(files, key=os.path.getsize)
    original = open(edited, encoding="utf-8").read()
    rows = []
    start = time.time()
    la = load()
    proc = subprocess.Popen(cmd, cwd=cwd, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1)
    try:
        def record(request, answer, wall):
            row = {"corpus": args.corpus, "kind": kind, "count": count, "round": rnd, "request": request, "ms": answer.get("ms", {}), "incremental": answer.get("incremental"), "ok": answer.get("ok"), "fallback": answer.get("fallback"), "wall": wall, "load": la, "edited": os.path.relpath(edited, cwd)}
            if not answer.get("ok") and request in ("first", "full"):
                row["error"] = json.dumps(answer.get("errors", [])[:1])[:300]
            rows.append(row)
            out.write(json.dumps(row) + "\n")
            out.flush()
        answer = read_answer(proc, args.seconds)
        record("first", answer, time.time() - start)
        appended = False
        for cycle in range(args.cycles):
            for r in (1, 2, 3):
                appended = not appended
                text = original + ("\n// ptyper-sessions\n" if appended else "")
                data = text.encode("utf-8")
                t0 = time.time()
                proc.stdin.write(f"text {edited} {len(data)}\n")
                proc.stdin.flush()
                proc.stdin.buffer.write(data)
                proc.stdin.write("build\n")
                proc.stdin.flush()
                answer = read_answer(proc, args.seconds)
                record(f"retype{r}", answer, time.time() - t0)
            t0 = time.time()
            proc.stdin.write("build\n")
            proc.stdin.flush()
            answer = read_answer(proc, args.seconds)
            record("full", answer, time.time() - t0)
        proc.stdin.write("quit\n")
        proc.stdin.flush()
        proc.wait(timeout=30)
        hows = [l.split(" ", 3)[3] for l in open(log).read().splitlines() if l.split(" ", 3)[2] != "end"]
        if count != "auto" and count > 1 and any(h not in (f"joined {count}", f"retried {count}") for h in hows):
            sys.exit(f"sessions: a full build at {count} workers was typed otherwise: {', '.join(hows)}")
        rows[0]["how"] = hows
    finally:
        if proc.poll() is None:
            proc.kill()
        shutil.rmtree(outdir, ignore_errors=True)
    return rows


def oneshot(args, cwd, inputs, flags, out):
    """`teq compiler check` of the corpus with `--time` at the automatic count against `--threads 1`, interleaved: the
    elapsed time, the type phase, and the `parallel attempt` row of a build that gave way and was typed again
    in a process of its own (`serial_again`)."""
    for rnd in range(args.rounds):
        for arm in (["auto", "one"] if rnd % 2 == 0 else ["one", "auto"]):
            env = {k: v for k, v in os.environ.items() if k not in ("TEQ_THREADS", "TEQ_SERIAL", "TEQ_SESSION_WORKERS", "TEQ_SESSION_THREADS", "TEQ_FORK")}
            extra = ["--threads", "1"] if arm == "one" else []
            la = load()
            t0 = time.time()
            r = subprocess.run([args.teq, "compiler", "check", *inputs, *flags, "--time", *extra], cwd=cwd, env=env, capture_output=True, text=True, timeout=args.seconds)
            wall = (time.time() - t0) * 1000
            m = re.search(r"^\s+type\s+([0-9.]+) ms", r.stderr, re.M)
            a = re.search(r"parallel attempt\s+([0-9.]+) ms", r.stderr)
            row = {"corpus": args.corpus, "kind": "oneshot", "count": arm, "round": rnd, "exit": r.returncode, "wall": wall, "type": float(m.group(1)) if m else None, "attempt": float(a.group(1)) if a else None, "note": "gave way" in r.stderr, "load": la}
            out.write(json.dumps(row) + "\n")
            out.flush()
            print(f"round {rnd} {arm}: wall {wall:.0f} ms, type {row['type']}, attempt {row['attempt']}, exit {r.returncode}, note {row['note']}, load {la:.1f}", flush=True)


def summary(paths):
    rows = [json.loads(l) for p in paths for l in open(p) if l.strip()]
    for corpus in sorted({r["corpus"] for r in rows if r["kind"] == "oneshot"}):
        for arm in ("one", "auto"):
            rs = [r for r in rows if r["kind"] == "oneshot" and r["corpus"] == corpus and r["count"] == arm]
            if rs:
                w = [r["wall"] for r in rs]
                a = [r["attempt"] for r in rs if r["attempt"]]
                print(f"{corpus:22} oneshot {arm:4} n {len(rs)} wall min {min(w):.0f} median {statistics.median(w):.0f}" + (f"; attempt min {min(a):.0f} median {statistics.median(a):.0f}" if a else ""))
    rows = [r for r in rows if r["kind"] != "oneshot"]
    groups = {}
    for r in rows:
        groups.setdefault((r["corpus"], r["kind"], r["request"]), {}).setdefault(r["count"], []).append(r)
    order = {"first": 0, "full": 1, "retype1": 2, "retype2": 3, "retype3": 4}
    print(f"{'corpus':22} {'kind':6} {'request':8} {'count':>5} {'n':>3} {'type min':>10} {'median':>10} {'total min':>10} {'ratio min':>9} {'ratio med':>9}  note")
    for key in sorted(groups, key=lambda k: (k[0], k[1], order.get(k[2], 9))):
        by = groups[key]
        base = by.get(1) or by.get("1")
        bt = [r["ms"].get("type", 0) for r in base] if base else None
        for count in sorted(by, key=str):
            rs = by[count]
            t = [r["ms"].get("type", 0) for r in rs]
            tot = [r["ms"].get("total", 0) for r in rs]
            note = []
            if any(not r.get("ok") for r in rs):
                note.append(f"{sum(not r.get('ok') for r in rs)} not ok")
            if key[2].startswith("retype") and any(not r.get("incremental") for r in rs):
                note.append(f"{sum(not r.get('incremental') for r in rs)} full")
            rmin = f"{min(t) / min(bt):.3f}" if bt and count != 1 else ""
            rmed = f"{statistics.median(t) / statistics.median(bt):.3f}" if bt and count != 1 else ""
            print(f"{key[0]:22} {key[1]:6} {key[2]:8} {count:>5} {len(rs):>3} {min(t):>10.1f} {statistics.median(t):>10.1f} {min(tot):>10.1f} {rmin:>9} {rmed:>9}  {'; '.join(note)}")


def main():
    p = argparse.ArgumentParser()
    p.add_argument("teq", nargs="?")
    p.add_argument("--out")
    p.add_argument("--summary", nargs="*")
    p.add_argument("--corpus", default="frontend")
    p.add_argument("--kind", default="check")
    p.add_argument("--counts", default="1,8")
    p.add_argument("--rounds", type=int, default=6)
    p.add_argument("--cycles", type=int, default=2)
    p.add_argument("--seconds", type=int, default=300)
    p.add_argument("--threads", action="store_true", help="give the count as --threads")
    p.add_argument("--app")
    p.add_argument("--app-root", default=os.environ.get("APP_ROOT"))
    p.add_argument("--corpus-dir")
    p.add_argument("--classpath", default="")
    p.add_argument("--env", action="append", default=[])
    p.add_argument("--extra", action="append", default=[], help="an argument more for the session (`description`)")
    p.add_argument("--oneshot", action="store_true", help="one-shot checks at the automatic count against --threads 1")
    args = p.parse_args()
    if args.summary is not None:
        summary(args.summary)
        return
    args.teq = os.path.abspath(args.teq)
    cwd, inputs, flags, split = corpus(args)
    counts = [c if c == "auto" else int(c) for c in args.counts.split(",")]
    if args.oneshot:
        with open(args.out, "a") as out:
            oneshot(args, cwd, inputs, flags, out)
        return
    with open(args.out, "a") as out:
        for rnd in range(args.rounds):
            order = counts if rnd % 2 == 0 else counts[::-1]
            for count in order:
                rows = session(args, cwd, inputs, flags, split, count, rnd, out)
                first = rows[0]
                print(f"round {rnd} count {count}: first {first['ms'].get('type', 0):.0f} ms" + (f" ({first.get('error', '')[:120]})" if not first.get("ok") else "") + "; full " + ", ".join(f"{r['ms'].get('type', 0):.0f}" for r in rows if r["request"] == "full") + "; retypes " + ", ".join(f"{r['ms'].get('type', 0):.1f}{'' if r['incremental'] else 'F'}" for r in rows if r["request"].startswith("retype")) + f"; load {first['load']:.1f}" + (f"; typed {', '.join(sorted(set(first.get('how', []))))}" if count == "auto" else ""), flush=True)


if __name__ == "__main__":
    main()
