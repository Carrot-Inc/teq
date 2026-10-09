#!/usr/bin/env python3
"""The parallel typer's measurements: the split of the type phase into
what stays sequential and what bodies do, the independence of the bodies, and the estimate for several
threads, from `teq compiler check --profile=<json>` on the core-only benchmark, the feature variants that
matter and, optionally, an application.

    python3 bench/ptyper.py [--teq target/release/teq] [--runs 3] [--out out/ptyper]
                            [--variants giv_4,inl_16,...] [--app <args file>]... [--json]
    python3 bench/ptyper.py --report out/ptyper/profiles

An args file names one program that is not a plain directory: its first line is the working
directory, the remaining lines the arguments of `teq` one per line (`build`, the sources, the
classpath, `--split`, ...); `--time` and `--profile=<file>` are appended. The report goes to
stdout as Markdown tables; `--json` writes every program's profile next to the report. The
profiles and the measured times are kept under `<out>/profiles`, and `--report <dir>` prints
the report of a measurement kept there without measuring again.
"""
import json
import os
import re
import statistics
import subprocess
import sys
import time

VARIANTS = ["giv_4", "inl_16", "ovl_16", "mac_16", "tm_16", "depth_32"]
THREADS = [4, 8, 16]
TIMEOUT = 300

# Which parts of the type phase the threads share and which stay sequential is the profile's
# say (`threads` per row, `Phase::on_threads` in src/typer/profile.rs). A lazy entry (made from
# inside a body) counts with the bodies, whatever its part; the merge's walk runs under
# --profile only and counts with neither.
MEASURED_ONLY = ["merge"]
# The threads' rows of a profile written before the flag (a reference build of an older binary).
THREADS_BEFORE_THE_FLAG = ["body", "macro", "class-check", "library-class-check", "deferred"]


def on_threads(row):
    return row.get("threads", row["phase"] in THREADS_BEFORE_THE_FLAG)


def parse_args(argv):
    opts = {"teq": "target/release/teq", "runs": 3, "out": "out/ptyper", "variants": VARIANTS, "apps": [], "json": False, "report": None}
    i = 0
    while i < len(argv):
        a = argv[i]
        if a == "--teq":
            opts["teq"] = argv[i + 1]
            i += 1
        elif a == "--runs":
            opts["runs"] = int(argv[i + 1])
            i += 1
        elif a == "--out":
            opts["out"] = argv[i + 1]
            i += 1
        elif a == "--variants":
            opts["variants"] = [v for v in argv[i + 1].split(",") if v]
            i += 1
        elif a == "--app":
            opts["apps"].append(argv[i + 1])
            i += 1
        elif a == "--json":
            opts["json"] = True
        elif a == "--report":
            opts["report"] = argv[i + 1]
            i += 1
        else:
            sys.exit(f"unknown argument {a}\n{__doc__}")
        i += 1
    return opts


def run(cmd, cwd=None):
    r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=TIMEOUT)
    return r.returncode, r.stdout + r.stderr


def ensure_programs(out):
    root = os.path.dirname(os.path.abspath(__file__))
    readme = os.path.join(out, "core-only")
    features = os.path.join(out, "features")
    if not os.path.isdir(readme):
        code, text = run(["python3", os.path.join(root, "gen.py"), readme, "50", "22"])
        if code != 0:
            sys.exit(f"gen.py failed: {text}")
    if not os.path.isdir(features):
        code, text = run(["python3", os.path.join(root, "features.py"), features])
        if code != 0:
            sys.exit(f"features.py failed: {text}")
    return readme, features


def type_ms(text):
    """The type phase of a `--time` report, in ms."""
    m = re.search(r"^phases\n(?:  .*\n)*?  type +([0-9.]+) ms", text, re.M)
    if m:
        return float(m.group(1))
    # A binary built before 2026-09-28 printed the phases on one line, a unit per value.
    m = re.search(r"type ([0-9.]+)(ms|s|µs)", text)
    return float(m.group(1)) * {"ms": 1, "s": 1000, "µs": 0.001}[m.group(2)] if m else None


class Program:
    def __init__(self, name, cwd, args):
        self.name = name
        self.cwd = cwd
        self.args = args
        self.plain_ms = []
        self.profiled_ms = []
        self.profile = None

    def measure(self, teq, runs, json_dir):
        for _ in range(runs):
            code, text = run([teq] + self.args + ["--time"], self.cwd)
            ms = type_ms(text)
            if code != 0 or ms is None:
                sys.exit(f"{self.name}: teq failed:\n{text[-2000:]}")
            self.plain_ms.append(ms)
        path = os.path.abspath(os.path.join(json_dir, f"{self.name}.json"))
        best = None
        for _ in range(runs):
            code, text = run([teq] + self.args + ["--time", f"--profile={path}"], self.cwd)
            ms = type_ms(text)
            if code != 0 or ms is None:
                sys.exit(f"{self.name}: teq --profile failed:\n{text[-2000:]}")
            self.profiled_ms.append(ms)
            with open(path) as f:
                prof = json.load(f)
            if best is None or prof["type_ns"] < best["type_ns"]:
                best = prof
        self.profile = best
        with open(path, "w") as f:
            json.dump(best, f)
        with open(os.path.join(json_dir, f"{self.name}.times.json"), "w") as f:
            json.dump({"cwd": self.cwd, "args": self.args, "plain_ms": self.plain_ms, "profiled_ms": self.profiled_ms}, f)

    @staticmethod
    def kept(json_dir, name):
        """A program measured earlier, from the profile and times kept under `json_dir`."""
        with open(os.path.join(json_dir, f"{name}.times.json")) as f:
            t = json.load(f)
        p = Program(name, t["cwd"], t["args"])
        p.plain_ms, p.profiled_ms = t["plain_ms"], t["profiled_ms"]
        with open(os.path.join(json_dir, f"{name}.json")) as f:
            p.profile = json.load(f)
        return p


def phases(prof):
    return {p["phase"]: p for p in prof.get("phases", [])}


def ms(ns):
    return ns / 1e6


def fmt_ms(ns):
    v = ns / 1e6
    if v >= 100:
        return f"{v:.0f}"
    if v >= 10:
        return f"{v:.1f}"
    return f"{v:.2f}"


def profiled_ns(prof):
    """The profiled type phase less the merge's walk, which runs under --profile only."""
    return prof["type_ns"] - sum(p["self_ns"] for p in prof.get("phases", []) if p["phase"] in MEASURED_ONLY)


def split_rows(prog):
    """The split of one program: (sequential ns, parallel ns, unattributed ns, per part)."""
    ph = phases(prog.profile)
    total = profiled_ns(prog.profile)
    seq = 0
    par = 0
    parts = {}
    for name, p in ph.items():
        eager = p["self_ns"] - p["lazy_self_ns"]
        lazy = p["lazy_self_ns"]
        parts[name] = (p["count"], p["self_ns"], p["lazy_count"], lazy)
        if name in MEASURED_ONLY:
            continue
        # A body typed before the walk, for a table of the signature phase, is the sequential
        # part's with everything under it, whatever the row.
        early = p.get("early_self_ns", 0)
        if on_threads(p):
            par += p["self_ns"] - early
            seq += early
        else:
            seq += eager
            par += lazy
    rest = total - seq - par
    return seq, par, rest, parts


def part_order(programs):
    """The rows in the profile's order, the sequential parts first."""
    seen = []
    for p in programs:
        for row in p.profile.get("phases", []):
            if row["phase"] not in seen:
                seen.append(row["phase"])
    threads = {row["phase"]: on_threads(row) for p in programs for row in p.profile.get("phases", [])}
    return [n for n in seen if not threads[n]] + [n for n in seen if threads[n]]


def lpt(sizes, threads):
    """The makespan of longest-first list scheduling of independent tasks."""
    loads = [0] * threads
    for s in sorted(sizes, reverse=True):
        i = min(range(threads), key=loads.__getitem__)
        loads[i] += s
    return max(loads) if loads else 0


def report(programs, opts):
    out = []
    w = out.append
    w("## The split of the type phase")
    w("")
    w("Self time of each part in ms, best profiled run; `lazy` is the share of a part entered from inside a body (a class or signature completed on demand, a std file entered), which a parallel typer does on the threads; the rows the threads share (the bodies, the macro expansions, the class checks, the bodies asked for) come after the sequential ones. The type phase is `teq compiler check --time` without `--profile`, best of the runs; the profiled column shows what the hooks cost.")
    w("")
    names = [p.name for p in programs]
    w("| part | " + " | ".join(names) + " |")
    w("|---|" + "---|" * len(names))
    order = part_order(programs)
    splits = {p.name: split_rows(p) for p in programs}
    for name in order:
        cells = []
        for p in programs:
            parts = splits[p.name][3]
            if name in parts:
                count, self_ns, lazy_count, lazy_ns = parts[name]
                cell = f"{fmt_ms(self_ns)} ({count})"
                if lazy_ns:
                    cell += f", lazy {fmt_ms(lazy_ns)} ({lazy_count})"
                cells.append(cell)
            else:
                cells.append("–")
        w(f"| {name} | " + " | ".join(cells) + " |")
    w("| unattributed | " + " | ".join(fmt_ms(splits[p.name][2]) for p in programs) + " |")
    w("| **sequential** | " + " | ".join(f"**{fmt_ms(splits[p.name][0] + splits[p.name][2])}**" for p in programs) + " |")
    w("| **on the threads** | " + " | ".join(f"**{fmt_ms(splits[p.name][1])}**" for p in programs) + " |")
    w("| type phase, profiled (less the merge's walk) | " + " | ".join(fmt_ms(profiled_ns(p.profile)) for p in programs) + " |")
    w("| type phase, `--time` (best of runs) | " + " | ".join(f"{min(p.plain_ms):.1f}" for p in programs) + " |")
    w("")
    w("## Body independence")
    w("")
    w("| | " + " | ".join(names) + " |")
    w("|---|" + "---|" * len(names))
    rows = [
        ("members' bodies typed", lambda b: b["count"]),
        ("in the walk over the files", lambda b: b["walk"]),
        ("before the walk, for the signature phase's tables (bodies, ms)", lambda b: f"{b.get('early', 0)} ({fmt_ms(b.get('early_self_ns', 0))})"),
        ("on demand from another body, for an inferred result type", lambda b: b["inferred_nested"]),
        ("on demand from another body, by a macro", lambda b: b["demand"]),
        ("for the reach pass or a macro after the walk", lambda b: b["reach"]),
        ("program / std / library", lambda b: f"{b['program']} / {b['std']} / {b['library']}"),
        ("bodies' own time, ms: walk / inferred / on demand / after the walk", lambda b: " / ".join(fmt_ms(b[k]) for k in ("walk_self_ns", "inferred_nested_self_ns", "demand_self_ns", "reach_self_ns"))),
        ("bodies' own time, ms: program / std / library", lambda b: " / ".join(fmt_ms(b[k]) for k in ("program_self_ns", "std_self_ns", "library_self_ns"))),
        ("definitions typed for an inferred result type", lambda b: b["inferred_defs"]),
        ("of them read from another body", lambda b: b["inferred_read"]),
        ("read before their body was typed", lambda b: b["inferred_read_open"]),
        ("bodies that read an inferred result", lambda b: f"{b['readers']} ({b['readers'] * 100 / max(b['count'], 1):.1f}%)"),
        ("of them one of another file", lambda b: b["readers_other_file"]),
        ("bodies that had to type another first", lambda b: b["blocked"]),
        ("independent bodies", lambda b: f"{b['count'] - b['readers']} ({(b['count'] - b['readers']) * 100 / max(b['count'], 1):.1f}%)"),
        ("inference chains (depth: bodies)", lambda b: ", ".join(f"{d}: {n}" for d, n in enumerate(b["chains"]) if d > 0 and n) or "none"),
        ("body time: median / p90 / p99 / max, µs", lambda b: f"{b['p50_ns'] / 1e3:.1f} / {b['p90_ns'] / 1e3:.1f} / {b['p99_ns'] / 1e3:.0f} / {b['max_ns'] / 1e3:.0f}"),
        ("largest body", lambda b: b["max_name"]),
        ("files with bodies", lambda b: len(b["per_file"])),
        ("largest file's bodies, ms", lambda b: fmt_ms(b["per_file"][0]["ns"]) + " (" + os.path.basename(b["per_file"][0]["file"]) + ")"),
    ]
    for label, f in rows:
        cells = []
        for p in programs:
            b = p.profile.get("bodies")
            cells.append(str(f(b)) if b else "–")
        w(f"| {label} | " + " | ".join(cells) + " |")
    w("")
    w("## The estimate")
    w("")
    w("Amdahl over the measured split: the sequential part as it is, the part on the threads divided by the thread count; then the same with the bodies scheduled largest first by file and by body over the measured sizes (the lazy completions and macro expansions counted inside the bodies that made them), which is what work distribution can reach without stealing. Times in ms against the unprofiled type phase (the profiled run's parts are scaled to it).")
    w("")
    w("| program | type phase | sequential | on threads | " + " | ".join(f"{t} threads: Amdahl / by file / by body" for t in THREADS) + " |")
    w("|---|---|---|---|" + "---|" * len(THREADS))
    for p in programs:
        seq, par, rest, _ = splits[p.name]
        scale = min(p.plain_ms) / ms(profiled_ns(p.profile))
        seq_ms = ms(seq + rest) * scale
        par_ms = ms(par) * scale
        b = p.profile.get("bodies") or {"per_file": [], "body_ns": []}
        # What the walk over the files did, per file and per body, inclusive of the lazy work.
        files = [f["ns"] for f in b["per_file"]]
        bodies_ns = b["body_ns"]
        body_total = sum(bodies_ns) or 1
        cells = []
        for t in THREADS:
            amdahl = seq_ms + par_ms / t
            by_file = seq_ms + par_ms * lpt(files, t) / body_total if files else float("nan")
            by_body = seq_ms + par_ms * lpt(bodies_ns, t) / body_total if bodies_ns else float("nan")
            cells.append(f"{amdahl:.1f} / {by_file:.1f} / {by_body:.1f}")
        w(f"| {p.name} | {min(p.plain_ms):.1f} | {seq_ms:.1f} | {par_ms:.1f} | " + " | ".join(cells) + " |")
    w("")
    for p in programs:
        a = p.profile.get("arenas")
        if a:
            w(f"- {p.name} arenas at the end of the type phase: " + ", ".join(f"{k} {v}" for k, v in a.items()))
    w("")
    w(f"Runs: {opts['runs']} per program and mode; profiled type phase against plain, per program: " + ", ".join(f"{p.name} {min(p.profiled_ms):.1f} / {min(p.plain_ms):.1f} ms" for p in programs))
    return "\n".join(out)


def main():
    opts = parse_args(sys.argv[1:])
    if opts["report"]:
        json_dir = opts["report"]
        names = sorted(f[: -len(".times.json")] for f in os.listdir(json_dir) if f.endswith(".times.json"))
        order = ["core-only"] + VARIANTS
        names.sort(key=lambda n: (order.index(n) if n in order else len(order), n))
        print(report([Program.kept(json_dir, n) for n in names], opts))
        return
    teq = os.path.abspath(opts["teq"])
    out = opts["out"]
    os.makedirs(out, exist_ok=True)
    readme, features = ensure_programs(out)
    programs = [Program("core-only", None, ["compiler", "check", readme])]
    for v in opts["variants"]:
        programs.append(Program(v, None, ["compiler", "check", os.path.join(features, v)]))
    for path in opts["apps"]:
        with open(path) as f:
            lines = [l.rstrip("\n") for l in f if l.strip()]
        name = os.path.splitext(os.path.basename(path))[0]
        programs.append(Program(name, lines[0], lines[1:]))
    json_dir = os.path.join(out, "profiles")
    os.makedirs(json_dir, exist_ok=True)
    t0 = time.time()
    for p in programs:
        p.measure(teq, opts["runs"], json_dir)
        print(f"# {p.name}: type {min(p.plain_ms):.1f} ms, profiled {min(p.profiled_ms):.1f} ms", file=sys.stderr)
    text = report(programs, opts)
    print(text)
    print(f"# measured in {time.time() - t0:.0f} s; profiles in {json_dir}", file=sys.stderr)


if __name__ == "__main__":
    main()
