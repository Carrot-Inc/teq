#!/usr/bin/env python3
"""The sbt route's compiler residents over bench/app's API side: one
`teq compiler watch --check --target jvm` session per module of the corpus's JVM side, as sbt-teq's compiler resident
starts one, over the module's upstream closure with `--own` its own directory, each started cold and its first
build timed; the seven summed per arm, the arms interleaved with their order turning each round.

    python3 bench/ptyper-residents.py --corpus-dir <bench/app/gen.py's output> --classpath <jvm jars>
        --out <file.jsonl> [--rounds 6] <name>=<teq>:<1|auto>...
    python3 bench/ptyper-residents.py --summary <file.jsonl>

An arm's count `n` is `--threads n`, and a session that did not type its first build as asked (`joined <n>`, or
for one worker anything but a retry) fails the run; `auto` gives no count, the overrides of the environment
cleared. Each line: the arm, the round, the module, the first build's `ms`, the wall time from the start to the
answer (the cold start), how the session typed it (`TEQ_SESSION_WORKERS_LOG`). The summary: per arm the sum of
the seven first builds' type phases and wall times, minimum and median over the rounds, and per module the count
it typed at.
"""
import argparse, json, os, statistics, subprocess, sys, tempfile, time

# The modules of the corpus's JVM side (bench/app/gen.py's MODULES, genlib/api.py), each with the modules its
# sources import, which its resident types too.
MODULES = {
    "shared/core": [],
    "shared/model": ["shared/core"],
    "api/infra": ["shared/core", "shared/model"],
    "api/business": ["shared/core", "shared/model", "api/infra"],
    "api/allocation": ["shared/core", "shared/model", "api/infra"],
    "api/public": ["shared/core", "shared/model", "api/infra"],
    "api/server": ["shared/core", "shared/model", "api/infra", "api/business", "api/allocation"],
}


def resident(args, teq, count, module, log):
    inputs = MODULES[module] + [module]
    threads = [] if count == "auto" else ["--threads", count]
    cmd = [teq, "compiler", "watch", "--check", "--target", "jvm", "--std=scala-library", *inputs, "--own", module,
           "--classpath", args.classpath, "--max-inlines", "80", *threads]
    env = {k: v for k, v in os.environ.items() if k not in ("TEQ_THREADS", "TEQ_SESSION_WORKERS", "TEQ_SESSION_THREADS", "TEQ_FORK")}
    env["TEQ_SESSION_WORKERS_LOG"] = log
    open(log, "w").close()
    t0 = time.time()
    p = subprocess.run(cmd, cwd=args.corpus_dir, env=env, input="quit\n", capture_output=True, text=True, timeout=300)
    wall = (time.time() - t0) * 1000
    answer = json.loads(p.stdout.splitlines()[0])
    how = " ".join(l.split(" ", 3)[3] for l in open(log).read().splitlines() if " end " not in l)
    if count not in ("auto", "1") and how != f"joined {count}":
        sys.exit(f"residents: {module} typed its first build as '{how}', not at the {count} workers asked")
    return {"ok": answer.get("ok"), "ms": answer.get("ms", {}), "wall": wall, "how": how}


def summary(path):
    rows = [json.loads(l) for l in open(path)]
    arms = sorted({r["arm"] for r in rows})
    for arm in arms:
        rs = [r for r in rows if r["arm"] == arm]
        key = lambda r: (r.get("run", 0), r["round"])
        rounds = sorted({key(r) for r in rs})
        sums = [sum(r["ms"].get("type", 0) for r in rs if key(r) == n) for n in rounds]
        walls = [sum(r["wall"] for r in rs if key(r) == n) for n in rounds]
        hows = {m: sorted({r["how"] for r in rs if r["module"] == m}) for m in MODULES}
        print(f"{arm}: {len(rounds)} rounds, the seven first builds' type phases summed min {min(sums):.0f} median {statistics.median(sums):.0f} ms; cold starts summed min {min(walls):.0f} median {statistics.median(walls):.0f} ms; not ok {sum(not r['ok'] for r in rs)}")
        for m in MODULES:
            t = [r["ms"].get("type", 0) for r in rs if r["module"] == m]
            print(f"  {m:16} {min(t):8.0f} ms min, {statistics.median(t):8.0f} median, typed {', '.join(hows[m])}")


def main():
    p = argparse.ArgumentParser()
    p.add_argument("arms", nargs="*")
    p.add_argument("--corpus-dir")
    p.add_argument("--classpath")
    p.add_argument("--out")
    p.add_argument("--rounds", type=int, default=6)
    p.add_argument("--summary")
    args = p.parse_args()
    if args.summary:
        return summary(args.summary)
    arms = []
    for a in args.arms:
        name, rest = a.split("=", 1)
        teq, count = rest.rsplit(":", 1)
        arms.append((name, os.path.abspath(os.path.expanduser(teq)), count))
    log = tempfile.mktemp(prefix="teq-residents-")
    started = int(time.time())
    with open(args.out, "a") as out:
        for rnd in range(args.rounds):
            order = arms if rnd % 2 == 0 else arms[::-1]
            for name, teq, count in order:
                total = 0
                for module in MODULES:
                    r = resident(args, teq, count, module, log)
                    out.write(json.dumps({"arm": name, "run": started, "round": rnd, "module": module, **r}) + "\n")
                    out.flush()
                    total += r["ms"].get("type", 0)
                print(f"round {rnd} {name}: the seven first builds' type phases {total:.0f} ms", flush=True)


if __name__ == "__main__":
    sys.exit(main())
