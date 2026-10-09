#!/usr/bin/env python3
"""The preparation's share of the type phase: the paired
intervention of the std's and the libraries' preparation replayed before the fork against the
build without it, the counts and the two arms interleaved run by run, every state give-way
bypassed (`TEQ_STATE_GIVEWAY=off`).

    python3 bench/ptyper-prep.py --app <build.json> --out <file.jsonl> [--runs 5]
        [--programs frontend,core-only] [--counts fork1,4,8,16] [--seconds 330]
        [--arms with,kinds:CPE,predict:classes] <teq>
    python3 bench/ptyper-prep.py --out <file.jsonl> --summary

Each program's keys are captured once by a plain build (`TEQ_PREP_CAPTURE`, `<out>.d/<program>
.keys`), then each run is `teq compiler check --time` with the keys replayed (`TEQ_PREP_REPLAY`) or not,
the arm that goes first alternating from one round to the next. A run's whole output is kept
(`<out>.d/<n>.txt`) and a line appended: the type phase, the replay's own time (its phase), the
workers' part, the merge, the loader's holds and waits in all, the holds' exclusive time by
category and the outermost holds by the category they were taken for. The build file's `root`
is read relative to the file.

An arm other than `without` is `with` (every key replayed), `kinds:<letters>` (the keys of those
kinds replayed, what they demand prepared with them) or `predict:<level>` (`TEQ_PREP_PREDICT`, the
diagnostic prediction). The summary gives per program and count the arms' type phases (minimum and median), the
replay's own time, and two numbers kept apart: the net saving, the phase without the replay less
the phase with it (the replay's serial time inside the phase), and the removable cost, the phase
without less the phase with the replay's own time taken out, a bound on what preparing the std
and the libraries anywhere can take out of the phase; both as the median of the rounds' paired
differences, with their least and greatest.
"""
import argparse, json, os, re, statistics, subprocess, sys, time

PREP = ["a std class checked", "a jar class converted", "a jar class checked", "a class completed (std or jar)",
        "an alias completed (std or jar)", "a signature completed (std or jar)", "a std or library body typed"]


def ms(s):
    n, u = re.match(r"([0-9.,]+)\s*(ms|s|µs)", s).groups()
    return float(n.replace(",", "")) * {"s": 1000, "ms": 1, "µs": 0.001}[u]


def app_args(path):
    d = json.load(open(path))
    a = list(d["sources"]) + ["--classpath", ":".join(d["classpath"])]
    if d.get("maxInlines"): a += ["--max-inlines", str(d["maxInlines"])]
    if d.get("strictEquality"): a += ["--strict-equality"]
    if d.get("kindProjector"): a += ["--kind-projector"]
    if d.get("werror"): a += ["--werror"]
    return os.path.join(os.path.dirname(os.path.abspath(path)), d["root"]), a


def parse(txt):
    rec = {}
    m = re.search(r"^  type\s+([0-9.,]+ (?:ms|s|µs))", txt, re.M)
    rec["type_ms"] = ms(m.group(1)) if m else None
    for key, pat in [("replay_ms", r"phase: the std and the libraries (?:prepared \(TEQ_PREP_REPLAY\)|predicted \(TEQ_PREP_PREDICT\))\s+([0-9.,]+ (?:ms|s|µs))"),
                     ("workers_ms", r"phase: workers, until the main worker's end\s+([0-9.,]+ (?:ms|s|µs))"),
                     ("walk_ms", r"phase: walk\s+([0-9.,]+ (?:ms|s|µs))"),
                     ("merge_ms", r"phase: merge\s+([0-9.,]+ (?:ms|s|µs))")]:
        m = re.search(pat, txt)
        if m:
            rec[key] = ms(m.group(1))
    for key, pat in [("holds", r"held the loader's lock: ([0-9,]+) holds, ([0-9.,]+ (?:ms|s|µs))"),
                     ("waits", r"waited for the loader's lock: ([0-9,]+) times, ([0-9.,]+ (?:ms|s|µs))")]:
        m = re.search(pat, txt)
        if m:
            rec[key] = [int(m.group(1).replace(",", "")), ms(m.group(2))]
    m = re.search(r"workers' time\s+\d+\s+([0-9.,]+ (?:ms|s|µs)) wall summed", txt)
    if m:
        rec["wall_summed_ms"] = ms(m.group(1))
    held, taken = {}, {}
    block = txt.split("taken for (the category", 1)
    for m in re.finditer(r"^      for (.+?): ([0-9,]+) stretches, ([0-9.,]+ (?:ms|s|µs))", block[0], re.M):
        held[m.group(1)] = ms(m.group(3))
    if len(block) > 1:
        for m in re.finditer(r"^      (.+?): ([0-9,]+) holds, ([0-9.,]+ (?:ms|s|µs))", block[1].split("    worker ", 1)[0], re.M):
            taken[m.group(1)] = [int(m.group(2).replace(",", "")), ms(m.group(3))]
    rec["held_by"] = held
    rec["taken_for"] = taken
    m = re.search(r"preparation replayed\s+([0-9,]+)\s+of (\d+) keys", txt)
    if m:
        rec["replayed"] = [int(m.group(1).replace(",", "")), int(m.group(2))]
    m = re.search(r"gave way|give-way asked", txt)
    rec["gave_way"] = bool(m)
    return rec


def summary(path):
    rows = [json.loads(l) for l in open(path)]
    order = ["plain", "fork1", "2", "4", "8", "16"]
    keys = sorted({(r["program"], r["config"]) for r in rows}, key=lambda k: (k[0], order.index(k[1]) if k[1] in order else 99))
    print("program    count  arm      type min / median   replay   held   waits   prep taken-for")
    for p, c in keys:
        rs = [r for r in rows if (r["program"], r["config"]) == (p, c) and r["type_ms"] is not None and r["exit"] == 0]
        arms = ["without"] + sorted({r["arm"] for r in rs} - {"without"})
        for arm in arms:
            a = [r for r in rs if r["arm"] == arm]
            if not a:
                continue
            t = [r["type_ms"] for r in a]
            rep = statistics.median([r.get("replay_ms", 0) for r in a])
            held = statistics.median([r.get("holds", [0, 0])[1] for r in a])
            waits = statistics.median([r.get("waits", [0, 0])[1] for r in a])
            prep = statistics.median([sum(r["taken_for"].get(k, [0, 0])[1] for k in PREP) for r in a])
            print(f"{p:10} {c:6} {arm:14} {min(t):8.0f} / {statistics.median(t):8.0f}  {rep:7.1f} {held:7.1f} {waits:7.1f} {prep:7.1f}  ({len(t)} runs, load {min(r['load'] for r in a)}-{max(r['load'] for r in a)})")
        by_round = {}
        for r in rs:
            by_round.setdefault(r["run"], {})[r["arm"]] = r
        for arm in arms[1:]:
            net, rem = [], []
            for pair in by_round.values():
                if arm in pair and "without" in pair:
                    w, o = pair[arm], pair["without"]
                    net.append(o["type_ms"] - w["type_ms"])
                    rem.append(o["type_ms"] - (w["type_ms"] - w.get("replay_ms", 0)))
            if net:
                base = statistics.median([r["type_ms"] for r in rs if r["arm"] == "without"])
                print(f"{'':10} {arm:14} net saving {statistics.median(net):7.1f} ms ({min(net):.0f} to {max(net):.0f}); removable {statistics.median(rem):7.1f} ms ({min(rem):.0f} to {max(rem):.0f}), {100 * statistics.median(rem) / base:.1f}% of the phase without ({len(net)} pairs)")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--app")
    ap.add_argument("--out", required=True)
    ap.add_argument("--runs", type=int, default=5)
    ap.add_argument("--programs", default="frontend,core-only")
    ap.add_argument("--counts", default="fork1,4,8,16")
    ap.add_argument("--core", default="out/core-only")
    ap.add_argument("--seconds", type=float, default=330)
    ap.add_argument("--arms", default="with")
    ap.add_argument("--summary", action="store_true")
    ap.add_argument("teq", nargs="?")
    a = ap.parse_args()
    if a.summary:
        return summary(a.out)
    teq = os.path.abspath(a.teq)
    if not os.path.isdir(a.core):
        subprocess.run(["python3", "bench/gen.py", a.core, "51", "22"], check=True, stdout=subprocess.DEVNULL, timeout=60)
    deadline = time.time() + a.seconds
    d = a.out + ".d"
    os.makedirs(d, exist_ok=True)
    n = len([f for f in os.listdir(d) if f.endswith(".txt")])
    programs = a.programs.split(",")
    where = {p: (app_args(a.app) if p == "frontend" else (os.getcwd(), [os.path.abspath(a.core)])) for p in programs}
    base = dict(os.environ, TEQ_STATE_GIVEWAY="off", TEQ_WORKERS_TIME="1")
    for v in ["TEQ_THREADS", "TEQ_FORK", "TEQ_HASH_GIVEWAY", "TEQ_PREP_REPLAY", "TEQ_PREP_CAPTURE", "TEQ_PREP_PREDICT"]:
        base.pop(v, None)
    arm_names = ["without"] + a.arms.split(",")
    for p in programs:
        keys = os.path.abspath(f"{d}/{p}.keys")
        if not os.path.exists(keys):
            cwd, args = where[p]
            subprocess.run([teq, "compiler", "check"] + args, cwd=cwd, env=dict(base, TEQ_PREP_CAPTURE=keys, TEQ_THREADS="1"), capture_output=True, timeout=120)
        for arm in arm_names:
            if arm.startswith("kinds:"):
                letters = arm.split(":", 1)[1]
                with open(f"{d}/{p}.{letters}.keys", "w") as out:
                    out.writelines(l for l in open(keys) if l[:1] in letters)

    def arm_env(env, p, arm):
        if arm == "with":
            env["TEQ_PREP_REPLAY"] = os.path.abspath(f"{d}/{p}.keys")
        elif arm.startswith("kinds:"):
            env["TEQ_PREP_REPLAY"] = os.path.abspath(f"{d}/{p}.{arm.split(':', 1)[1]}.keys")
        elif arm.startswith("predict:"):
            env["TEQ_PREP_PREDICT"] = arm.split(":", 1)[1]
    done = len([l for l in open(a.out)]) if os.path.exists(a.out) else 0
    started = int(time.time())
    with open(a.out, "a") as f:
        for r in range(a.runs):
            for p in programs:
                cwd, args = where[p]
                for c in a.counts.split(","):
                    shift = (r + done) % len(arm_names)
                    arms = arm_names[shift:] + arm_names[:shift]
                    for arm in arms:
                        if time.time() > deadline:
                            print("TIMEOUT: the bound of --seconds was reached", flush=True)
                            sys.exit(3)
                        env = dict(base, TEQ_THREADS="1" if c in ("plain", "fork1") else c)
                        if c == "fork1":
                            env["TEQ_FORK"] = "1"
                        arm_env(env, p, arm)
                        load = os.getloadavg()[0]
                        t0 = time.time()
                        res = subprocess.run([teq, "compiler", "check"] + args + ["--time"], cwd=cwd, env=env, capture_output=True, text=True, timeout=120)
                        txt = res.stdout + res.stderr
                        n += 1
                        open(f"{d}/{n}.txt", "w").write(txt)
                        rec = {"n": n, "run": f"{started}-{r}", "round": r, "program": p, "config": c, "arm": arm, "exit": res.returncode,
                               "wall": round(time.time() - t0, 3), "load": round(load, 2)}
                        rec.update(parse(txt))
                        f.write(json.dumps(rec) + "\n")
                        f.flush()
                        print(json.dumps({k: rec[k] for k in ["n", "round", "program", "config", "arm", "exit", "type_ms", "load"] if k in rec} | {"replay_ms": rec.get("replay_ms")}), flush=True)


main()
