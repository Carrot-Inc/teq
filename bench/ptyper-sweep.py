#!/usr/bin/env python3
"""The typer's fork against the program's size: `teq compiler check --time` of
programs of a range of sizes at one worker and at worker counts, interleaved run by run, for the size
threshold of the automatic count.

    python3 bench/ptyper-sweep.py --out <file.jsonl> [--sets budget,gen,app] [--configs plain,2,4,8,12,16]
        [--runs 3] [--only <regex>] [--seconds 330] <teq>
    python3 bench/ptyper-sweep.py --out <file.jsonl> --summary [--counts 4,8]

The sets: `budget`, the programs of bench/programs.sh (generated under out/budget); `gen`, `bench/gen.py
<n> 22` at 1 to 64 files (under out/sweep); `app`, the application corpus's shared and frontend trees and
its shared and api trees and its shared tree alone by `bench/app/gen.py --scale` from 1/16 to 1 (under out/sweep; at a half and at 0.375 the generator's shared tree names an enum case it does not define), the frontend's class
catalog declared a cache (`--cacheable-state meridian.web.css.Catalog`, as the application declares its own). Each program's
size is measured as the build selects it: the UTF-8 bytes of its `.scala` files under its source
arguments, each file once, and their count. `plain` is `TEQ_THREADS=1`, a number `TEQ_THREADS=<n>`, and
`auto@<bytes>` no count, the automatic rule applied with its threshold at those bytes (`TEQ_TYPER_THRESHOLD`),
the count it chose recorded; the state give-way at its default. Each run's line: the type phase, the elapsed
time, the workers started, whether it gave way. The summary gives per program its size and per count the ratio of the minimum type phase to
the plain build's (a build that gave way at every run: its elapsed time's to the plain build's), and per
count the smallest size from which every program pays (its ratio under 1).
"""
import argparse, json, os, re, statistics, subprocess, sys, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GEN_FILES = [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64]
APP_SCALES = ["0.0625", "0.125", "0.25", "0.6", "0.75", "1"]


def ms(s):
    n, u = re.match(r"([0-9.,]+)\s*(ms|s|µs)", s).groups()
    return float(n.replace(",", "")) * {"s": 1000, "ms": 1, "µs": 0.001}[u]


def size_of(args):
    seen, total, n = set(), 0, 0
    for a in args:
        if a.startswith("--"):
            break
        p = a if os.path.isabs(a) else os.path.join(ROOT, a)
        paths = [p] if os.path.isfile(p) else [os.path.join(d, f) for d, ds, fs in os.walk(p) for f in fs]
        for f in paths:
            if f.endswith(".scala") and not os.path.basename(f).startswith("."):
                real = os.path.realpath(f)
                if real not in seen:
                    seen.add(real)
                    total += len(open(real, "rb").read())
                    n += 1
    return total, n


def budget_programs():
    out = subprocess.run(["bash", "-c", 'work=out/budget; . bench/programs.sh > /dev/null || exit 1; for p in $programs; do printf "%s\\t%s\\n" "$p" "$(program_args $p)"; done'],
                         cwd=ROOT, capture_output=True, text=True, timeout=300)
    if out.returncode != 0:
        sys.exit("sweep: bench/programs.sh failed: " + out.stderr[-300:])
    return [(name, args.split()) for name, args in (l.split("\t", 1) for l in out.stdout.splitlines())]


def gen_programs():
    progs = []
    for n in GEN_FILES:
        d = os.path.join(ROOT, f"out/sweep/gen-{n}")
        if not os.path.isdir(d):
            subprocess.run(["python3", "bench/gen.py", d, str(n), "22"], cwd=ROOT, check=True, stdout=subprocess.DEVNULL, timeout=60)
        progs.append((f"gen-{n}", [d]))
    return progs


def app_programs():
    cp = subprocess.run(["bash", "-c", '. tests/support/jars.sh; for n in scala-library cats-kernel cats-core sourcecode; do jar_of $n; done'],
                        cwd=ROOT, capture_output=True, text=True, timeout=60).stdout.split()
    progs = []
    for s in APP_SCALES:
        d = os.path.join(ROOT, f"out/sweep/app-{s}")
        if not os.path.isdir(d):
            subprocess.run(["python3", "bench/app/gen.py", d, "--scale", s], cwd=ROOT, check=True, stdout=subprocess.DEVNULL, timeout=120)
        progs.append((f"app-frontend-{s}", [f"{d}/shared", f"{d}/frontend", "--classpath", ":".join(cp), "--cacheable-state", "meridian.web.css.Catalog"]))
        progs.append((f"app-api-{s}", [f"{d}/shared", f"{d}/api", "--classpath", ":".join(cp)]))
        progs.append((f"app-shared-{s}", [f"{d}/shared", "--classpath", ":".join(cp)]))
    return progs


def summary(path, counts):
    rows = [json.loads(l) for l in open(path)]
    progs = {}
    for r in rows:
        progs.setdefault(r["program"], {"bytes": r["bytes"], "files": r["files"], "runs": {}})["runs"].setdefault(r["config"], []).append(r)
    configs = sorted({r["config"] for r in rows if r["config"] != "plain"}, key=lambda c: (c.startswith("auto@"), int(c.split("@")[-1])))
    print(f"{'program':22} {'bytes':>9} {'files':>5} {'plain':>8} " + " ".join(f"{c:>13}" for c in configs))
    table = []
    for name, p in sorted(progs.items(), key=lambda kv: kv[1]["bytes"]):
        plain = [r["type_ms"] for r in p["runs"].get("plain", []) if r["type_ms"] is not None and not r["gave_way"]]
        if not plain:
            continue
        base = min(plain)
        cells, ratios = [], {}
        for c in configs:
            t = [r["type_ms"] for r in p["runs"].get(c, []) if r["type_ms"] is not None and not r["gave_way"]]
            gave = [r["wall"] for r in p["runs"].get(c, []) if r["gave_way"]]
            if t:
                ratios[c] = min(t) / base
                workers = sorted({r["workers"] for r in p["runs"].get(c, []) if not r["gave_way"]})
                chosen = f"@{'/'.join(map(str, workers))}" if c.startswith("auto@") else ""
                cells.append(f"{min(t):7.0f} {ratios[c]:4.2f}{'g' if gave else ' '}{chosen}")
            elif gave:
                walls = [r["wall"] for r in p["runs"].get("plain", [])]
                cells.append(f"gave way {min(gave) / min(walls):4.2f}")
            else:
                cells.append(f"{'-':>13}")
        table.append((name, p["bytes"], p["files"], base, ratios))
        print(f"{name:22} {p['bytes']:9} {p['files']:5} {base:8.1f} " + " ".join(cells))
    for c in counts:
        losing = [t for t in table if c in t[4] and t[4][c] >= 1.0]
        paying = [t for t in table if c in t[4] and t[4][c] < 1.0]
        above = max((t[1] for t in losing), default=0)
        print(f"at {c}: {len(paying)} of {len(paying) + len(losing)} pay; the largest that does not pays nothing above it: {above} bytes"
              + (f" ({max(losing, key=lambda t: t[1])[0]})" if losing else ""))
        by_files = max((t[2] for t in losing), default=0)
        print(f"       by files: the most files of one that does not pay: {by_files}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--sets", default="budget,gen,app")
    ap.add_argument("--configs", default="plain,2,4,8,12,16")
    ap.add_argument("--runs", type=int, default=3)
    ap.add_argument("--only")
    ap.add_argument("--seconds", type=float, default=330)
    ap.add_argument("--summary", action="store_true")
    ap.add_argument("--counts", default="2,4,8,16")
    ap.add_argument("teq", nargs="?")
    a = ap.parse_args()
    if a.summary:
        return summary(a.out, a.counts.split(","))
    teq = os.path.abspath(a.teq)
    deadline = time.time() + a.seconds
    progs = []
    for s in a.sets.split(","):
        progs += {"budget": budget_programs, "gen": gen_programs, "app": app_programs}[s]()
    if a.only:
        progs = [p for p in progs if re.search(a.only, p[0])]
    sizes = {name: size_of(args) for name, args in progs}
    started = int(time.time())
    with open(a.out, "a") as f:
        for r in range(a.runs):
            for name, args in progs:
                for c in a.configs.split(","):
                    if time.time() > deadline:
                        print("TIMEOUT: the bound of --seconds was reached", flush=True)
                        sys.exit(3)
                    env = dict(os.environ)
                    for v in ["TEQ_THREADS", "TEQ_FORK", "TEQ_SERIAL", "TEQ_STATE_GIVEWAY", "TEQ_SERIAL_TRACE", "TEQ_TYPER_THRESHOLD"]:
                        env.pop(v, None)
                    if c.startswith("auto@"):
                        env["TEQ_TYPER_THRESHOLD"] = c.split("@")[1]
                    else:
                        env["TEQ_THREADS"] = "1" if c == "plain" else c
                    load = os.getloadavg()[0]
                    t0 = time.time()
                    res = subprocess.run([teq, "compiler", "check"] + args + ["--time"], cwd=ROOT, env=env, capture_output=True, text=True, timeout=120)
                    wall = time.time() - t0
                    txt = res.stdout + res.stderr
                    m = re.search(r"^  type\s+([0-9.,]+ (?:ms|s|µs))", txt, re.M)
                    w = re.search(r"^  workers' time\s+(\d+)\s", txt, re.M)
                    rec = {"run": f"{started}-{r}", "program": name, "config": c, "bytes": sizes[name][0], "files": sizes[name][1],
                           "type_ms": ms(m.group(1)) if m else None, "wall": round(wall, 3), "exit": res.returncode,
                           "workers": int(w.group(1)) if w else 1, "gave_way": "gave way to one worker" in txt, "load": round(load, 2)}
                    f.write(json.dumps(rec) + "\n")
                    f.flush()
                    print(json.dumps(rec), flush=True)


main()
