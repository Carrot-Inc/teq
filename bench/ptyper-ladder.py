#!/usr/bin/env python3
"""The parallel typer's ladder: `teq compiler check --time`
of the programs asked for at one worker without the fork, one worker through the fork, the worker counts
asked for and the automatic selection, under the policies asked for (the default, or the state give-way
bypassed, `TEQ_STATE_GIVEWAY=off`), the binaries, the policies and the configurations interleaved run by
run.

    python3 bench/ptyper-ladder.py --out <file.jsonl> [--runs 5] [--app <build.json>]
        [--build <name>=<build.json>]... [--programs frontend,core-only]
        [--configs plain,fork1,auto,2,4,8,16] [--policies bypassed] [--seconds 330] label=<teq>...

A program is `core-only` (`bench/gen.py 51 22` into `--core`) or a build file named by `--build`
(`--app` is `--build frontend=<file>`): its `root` (relative to the file), `sources`, `classpath`,
the flags of the export's description (`maxInlines`, `strictEquality`, `kindProjector`, `werror`,
`cacheableState`) and `args`, more arguments as given. Every configuration selects its count: `plain`
is `TEQ_THREADS=1`, `fork1` `TEQ_THREADS=1` with `TEQ_FORK=1`, a number `TEQ_THREADS=<n>`, and `auto`
neither, the count the binary selects by itself. Each run's whole output is kept beside the jsonl
(<out>.d/<n>.txt) and a line appended: the type phase, the process's elapsed time (a build that gives
way includes its discarded attempt there and not in the type phase), the attempt's own time, the
count the build typed with, the phases of the workers' section and the merge's and the fork's parts,
the loader's holds and waits, the load. `--summary` prints the minimum and median per program,
configuration, policy and binary of a jsonl written before; `--phases` adds the phases.
"""
import argparse, json, os, re, statistics, subprocess, sys, time

ORDER = ["plain", "fork1", "auto", "1", "2", "4", "8", "12", "16", "32"]
PHASES = ["signatures", "prefix, the std files", "prefix, the macro reach", "fork", "workers, until the main worker's end",
          "workers, the others joined", "merge", "final"]


def ms(s):
    n, u = re.match(r"([0-9.,]+)\s*(ms|s|µs)", s).groups()
    return float(n.replace(",", "")) * {"s": 1000, "ms": 1, "µs": 0.001}[u]


def build_args(path):
    d = json.load(open(path))
    a = list(d["sources"])
    if d.get("classpath"): a += ["--classpath", ":".join(d["classpath"])]
    if d.get("maxInlines"): a += ["--max-inlines", str(d["maxInlines"])]
    if d.get("strictEquality"): a += ["--strict-equality"]
    if d.get("kindProjector"): a += ["--kind-projector"]
    if d.get("werror"): a += ["--werror"]
    for n in d.get("cacheableState", []): a += ["--cacheable-state", n]
    a += d.get("args", [])
    return os.path.join(os.path.dirname(os.path.abspath(path)), d["root"]), a


def config_env(env, c):
    for v in ["TEQ_SERIAL_TRACE", "TEQ_THREADS", "TEQ_FORK", "TEQ_SERIAL"]:
        env.pop(v, None)
    if c == "plain":
        env["TEQ_THREADS"] = "1"
    elif c == "fork1":
        env["TEQ_THREADS"] = "1"
        env["TEQ_FORK"] = "1"
    elif c != "auto":
        env["TEQ_THREADS"] = c
    return env


def parse(txt):
    rec = {}
    m = re.search(r"^  type\s+([0-9.,]+ (?:ms|s|µs))", txt, re.M)
    rec["type_ms"] = ms(m.group(1)) if m else None
    m = re.search(r"parallel attempt\s+([0-9.,]+ (?:ms|s|µs))", txt)
    if m:
        rec["attempt_ms"] = ms(m.group(1))
    m = re.search(r"^  workers' time\s+(\d+)\s", txt, re.M)
    rec["workers"] = int(m.group(1)) if m else 1
    phases = {}
    for name in PHASES:
        m = re.search(r"phase: " + re.escape(name) + r"\s+([0-9.,]+ (?:ms|s|µs))", txt)
        if m:
            phases[name] = ms(m.group(1))
    for m in re.finditer(r"^  ((?:merge|fork) [a-z' ]+?)\s{2,}([0-9.,]+ (?:ms|s|µs))", txt, re.M):
        phases[m.group(1)] = ms(m.group(2))
    rec["phases"] = phases
    for key, pat in [("holds", r"held the loader's lock: ([0-9,]+) holds, ([0-9.,]+ (?:ms|s|µs))"), ("waits", r"waited for the loader's lock: ([0-9,]+) times, ([0-9.,]+ (?:ms|s|µs))")]:
        m = re.search(pat, txt)
        if m:
            rec[key] = [int(m.group(1).replace(",", "")), ms(m.group(2))]
    if "merge" in phases:
        rec["merge_ms"] = phases["merge"]
    return rec


def summary(path, with_phases):
    rows = [json.loads(l) for l in open(path)]
    key = lambda r: (r["program"], r["config"], r.get("policy", "bypassed"), r["bin"])
    for k in sorted({key(r) for r in rows}, key=lambda k: (k[0], ORDER.index(k[1]) if k[1] in ORDER else 99, k[2], k[3])):
        sel = [r for r in rows if key(r) == k and r["type_ms"] is not None]
        if not sel:
            continue
        gave = [r for r in sel if r.get("gave_way")]
        done = [r for r in sel if not r.get("gave_way")]
        counts = sorted({r.get("workers", 1) for r in done})
        line = f"{k[0]:10} {k[1]:6} {k[2]:9} {k[3]:10}"
        if done:
            t = [r["type_ms"] for r in done]
            w = [r["wall"] * 1000 for r in done]
            line += f" type {min(t):8.0f} {statistics.median(t):8.0f}  elapsed {min(w):8.0f} {statistics.median(w):8.0f}"
        if gave:
            w = [r["wall"] * 1000 for r in gave]
            a = [r.get("attempt_ms", 0) for r in gave]
            line += f"  gave way {len(gave)}: elapsed {min(w):.0f} {statistics.median(w):.0f}, attempt {min(a):.0f} {statistics.median(a):.0f}"
        line += f"  ({len(sel)} runs, {'/'.join(map(str, counts)) or '-'} workers, load {min(r['load'] for r in sel)}-{max(r['load'] for r in sel)})"
        print(line)
        if with_phases and done:
            names = [n for n in PHASES if any(n in r["phases"] for r in done)] + sorted({n for r in done for n in r["phases"] if n not in PHASES})
            for n in names:
                v = [r["phases"][n] for r in done if n in r["phases"]]
                if v and max(v) >= 0.05:
                    print(f"{'':12} {n:48} {min(v):8.1f} {statistics.median(v):8.1f}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--app")
    ap.add_argument("--build", action="append", default=[])
    ap.add_argument("--out", required=True)
    ap.add_argument("--runs", type=int, default=5)
    ap.add_argument("--programs", default="frontend,core-only")
    ap.add_argument("--configs", default="plain,fork1,auto,2,4,8,16")
    ap.add_argument("--policies", default="bypassed")
    ap.add_argument("--core", default="out/core-only")
    ap.add_argument("--seconds", type=float, default=330)
    ap.add_argument("--summary", action="store_true")
    ap.add_argument("--phases", action="store_true")
    ap.add_argument("bins", nargs="*")
    a = ap.parse_args()
    if a.summary:
        return summary(a.out, a.phases)
    builds = dict(b.split("=", 1) for b in a.build)
    if a.app:
        builds["frontend"] = a.app
    programs = a.programs.split(",")
    for p in programs:
        if p != "core-only" and p not in builds:
            sys.exit(f"ladder: no build file for {p} (--build {p}=<file>)")
    if "core-only" in programs and not os.path.isdir(a.core):
        subprocess.run(["python3", "bench/gen.py", a.core, "51", "22"], check=True, stdout=subprocess.DEVNULL, timeout=60)
    where = {p: build_args(builds[p]) if p != "core-only" else (os.getcwd(), [os.path.abspath(a.core)]) for p in programs}
    bins = [b.split("=", 1) for b in a.bins]
    deadline = time.time() + a.seconds
    os.makedirs(a.out + ".d", exist_ok=True)
    n = len(os.listdir(a.out + ".d"))
    started = int(time.time())
    with open(a.out, "a") as f:
        for r in range(a.runs):
            for p in programs:
                cwd, args = where[p]
                for c in a.configs.split(","):
                  for policy in a.policies.split(","):
                    for label, b in bins:
                        if time.time() > deadline:
                            print("TIMEOUT: the bound of --seconds was reached", flush=True)
                            sys.exit(3)
                        env = config_env(dict(os.environ), c)
                        if policy == "bypassed":
                            env["TEQ_STATE_GIVEWAY"] = "off"
                        else:
                            env.pop("TEQ_STATE_GIVEWAY", None)
                        load = os.getloadavg()[0]
                        t0 = time.time()
                        res = subprocess.run([os.path.abspath(b), "compiler", "check"] + args + ["--time"], cwd=cwd, env=env, capture_output=True, text=True, timeout=120)
                        wall = time.time() - t0
                        txt = res.stdout + res.stderr
                        n += 1
                        open(f"{a.out}.d/{n}.txt", "w").write(txt)
                        rec = {"n": n, "run": f"{started}-{r}", "program": p, "config": c, "policy": policy, "bin": label,
                               "gave_way": "gave way to one worker" in txt, "exit": res.returncode, "wall": round(wall, 3), "load": round(load, 2)}
                        rec.update(parse(txt))
                        f.write(json.dumps(rec) + "\n")
                        f.flush()
                        print(json.dumps({k: v for k, v in rec.items() if k not in ("phases",)}), flush=True)

main()
