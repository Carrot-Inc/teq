#!/usr/bin/env python3
"""The measuring half of bench/runtime.sh.

  measure <results.tsv> <size> <program> <toolchain> <flags> <run> <expected> <timeout> -- cmd...
      runs the command once under `timeout`, takes its wall time and peak RSS from wait4, reads the
      program's own `bench ... first_ns= total_ns= steady_ns=` line from stderr, checks stdout
      against the expected output and appends one row to the results file; a row that is already
      there for the same size, program, toolchain, flags and run is kept and nothing runs, unless
      FRESH is set in the environment
  gc <results.tsv> <size> <program> <toolchain> <flags> <timeout> -- cmd...
      runs the command once with GC logging (node --trace-gc, java -Xlog:gc, put before the
      script or the class path) and records the number of collections as a row with run "gc",
      with the same skip
  report <out dir> <report.md> [key=value ...]
      aggregates every <out dir>/*/results.tsv into Markdown tables; the key=value pairs (machine,
      versions, flags) are printed as the header
  budget <results.tsv> <budgets file> <tolerance %> [record]
      compares the quick-size rows of teq's own toolchains with the recorded budgets, or records
      them; exits 1 when a number regressed by more than the tolerance
  cpuprofile <file.cpuprofile> [top]
      the functions of a node --cpu-prof recording with the most self time, ten by default
  jfr <file.jfr> [top]
      the methods on top of the most jdk.ExecutionSample stacks of a JFR recording, through
      `jfr print`

A row: size, program, toolchain, flags, run, wall_ms, rss_mb, first_ms, steady_ms, total_ms,
exit, load1, gc, when. Times are milliseconds; the load is the one-minute average at the start
of the run, so that a noisy machine shows in the data.
"""
import datetime
import os
import re
import signal
import statistics
import subprocess
import sys
import threading
import time

COLUMNS = ["size", "program", "toolchain", "flags", "run", "wall_ms", "rss_mb", "first_ms",
           "steady_ms", "total_ms", "exit", "load1", "gc", "when"]
TOOLCHAINS = ["teq-dev", "teq-release", "sjs-dev", "sjs-full", "sjs-es-dev", "sjs-es-full",
              "teq-jvm", "scalac", "scalac-opt"]
NODE = {"teq-dev", "teq-release", "sjs-dev", "sjs-full", "sjs-es-dev", "sjs-es-full"}
SUMMARY = [("teq-dev", "node"), ("teq-release", "node"), ("sjs-full", "node"), ("sjs-es-full", "node"),
           ("teq-jvm", "teq"), ("scalac", "teq"), ("scalac-opt", "teq")]
SUMMARY_DEFAULT = [("teq-dev", "node"), ("teq-release", "node"), ("sjs-full", "node"), ("sjs-es-full", "node"),
                   ("teq-jvm", "default"), ("scalac", "default"), ("scalac-opt", "default")]
BUDGETED = [("teq-dev", "node"), ("teq-jvm", "teq")]


def load1():
    try:
        return os.getloadavg()[0]
    except OSError:
        return 0.0


def run_once(cmd, timeout, out_path, err_path):
    """Wall time, exit code and peak RSS in MB of one run of cmd, killed after `timeout` seconds (exit 124).
    The bound is kept here: the remote machines' timeout(1), uutils', polls its child every 100 ms, which rounds
    every wall time measured through it up to the next 100 ms."""
    with open(out_path, "wb") as out, open(err_path, "wb") as err:
        start = time.perf_counter()
        p = subprocess.Popen(cmd, stdout=out, stderr=err, start_new_session=True)
        expired = threading.Event()
        timer = threading.Timer(float(timeout), lambda: (expired.set(), os.killpg(p.pid, signal.SIGKILL)))
        timer.start()
        _, status, usage = os.wait4(p.pid, 0)
        wall = time.perf_counter() - start
        timer.cancel()
    p.returncode = 124 if expired.is_set() else os.waitstatus_to_exitcode(status)
    rss = usage.ru_maxrss / (1024 * 1024) if sys.platform == "darwin" else usage.ru_maxrss / 1024
    return wall * 1000, p.returncode, rss


def bench_line(err_path):
    text = open(err_path, "rb").read().decode("utf-8", "replace")
    m = re.search(r"^bench \S+ n=\d+ k=\d+ first_ns=(\d+) total_ns=(\d+) steady_ns=(\d+)", text, re.M)
    if not m:
        return None
    return tuple(int(m.group(i)) / 1e6 for i in (1, 2, 3))


def append(results, row):
    new = not os.path.exists(results) or os.path.getsize(results) == 0
    with open(results, "a") as f:
        if new:
            f.write("\t".join(COLUMNS) + "\n")
        f.write("\t".join(str(row[c]) for c in COLUMNS) + "\n")


def split_cmd(args):
    i = args.index("--")
    return args[:i], args[i + 1:]


def done(results, size, program, toolchain, flags, run):
    if os.environ.get("FRESH") or not os.path.exists(results):
        return False
    return any(r["size"] == size and r["program"] == program and r["toolchain"] == toolchain
               and r["flags"] == flags and r["run"] == run and r["exit"] == "0" for r in read_rows(results))


def measure(args):
    head, cmd = split_cmd(args)
    results, size, program, toolchain, flags, run, expected, timeout = head
    if done(results, size, program, toolchain, flags, run):
        return 0
    base = os.path.join(os.path.dirname(results), program, f"{toolchain}.{flags}")
    load = load1()
    wall, code, rss = run_once(cmd, timeout, base + ".run.out", base + ".run.err")
    times = bench_line(base + ".run.err")
    same = open(base + ".run.out", "rb").read() == open(expected, "rb").read()
    if code != 0 or times is None or not same:
        why = f"exit {code}" if code != 0 else ("no bench line on stderr" if times is None else "output differs from the expected")
        print(f"FAIL {size} {program} {toolchain} {flags} run {run}: {why} (see {base}.run.err)")
        code = code or 1
        times = times or (0, 0, 0)
    first, total, steady = times
    append(results, {"size": size, "program": program, "toolchain": toolchain, "flags": flags, "run": run,
                     "wall_ms": f"{wall:.1f}", "rss_mb": f"{rss:.1f}", "first_ms": f"{first:.2f}",
                     "steady_ms": f"{steady:.2f}", "total_ms": f"{total:.2f}", "exit": code,
                     "load1": f"{load:.2f}", "gc": "", "when": datetime.datetime.now().isoformat(timespec="seconds")})
    print(f"{size} {program} {toolchain} {flags} run {run}: wall {wall:.0f} ms, first {first:.1f} ms, steady {steady:.2f} ms, rss {rss:.0f} MB, load {load:.1f}")
    return 0 if code == 0 else 1


def gc(args):
    head, cmd = split_cmd(args)
    results, size, program, toolchain, flags, timeout = head
    if done(results, size, program, toolchain, flags, "gc"):
        return 0
    base = os.path.join(os.path.dirname(results), program, f"{toolchain}.{flags}.gc")
    _, code, _ = run_once(cmd, timeout, base + ".out", base + ".err")
    text = open(base + ".out", "rb").read().decode("utf-8", "replace") + open(base + ".err", "rb").read().decode("utf-8", "replace")
    if toolchain in NODE:
        count = len(re.findall(r"ms: (?:Scavenge|Mark-Compact|Mark-sweep|Minor Mark-Sweep|Minor Mark-Compact)", text))
    else:
        count = len(re.findall(r"GC\(\d+\) Pause", text))
    append(results, {"size": size, "program": program, "toolchain": toolchain, "flags": flags, "run": "gc",
                     "wall_ms": "", "rss_mb": "", "first_ms": "", "steady_ms": "", "total_ms": "", "exit": code,
                     "load1": f"{load1():.2f}", "gc": count, "when": datetime.datetime.now().isoformat(timespec="seconds")})
    print(f"{size} {program} {toolchain} {flags} gc: {count} collections" + ("" if code == 0 else f" (exit {code})"))
    return 0 if code == 0 else 1


def read_rows(path):
    rows = []
    with open(path) as f:
        header = None
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if header is None:
                header = parts
                continue
            if len(parts) == len(header):
                rows.append(dict(zip(header, parts)))
    return rows


def median(values):
    return statistics.median(values) if values else None


def summarize(rows):
    """Per (size, program, toolchain, flags): medians, extremes and the noise of the timed runs,
    and the GC count of the gc run; the latest run of each index wins, so a rerun replaces."""
    latest = {}
    gcs = {}
    for r in rows:
        key = (r["size"], r["program"], r["toolchain"], r["flags"])
        if r["run"] == "gc":
            gcs[key] = int(r["gc"])
        elif r["exit"] == "0":
            latest[(key, r["run"])] = r
    cells = {}
    for (key, _), r in latest.items():
        cells.setdefault(key, []).append(r)
    out = {}
    for key, rs in cells.items():
        walls = [float(r["wall_ms"]) for r in rs]
        steadies = [float(r["steady_ms"]) for r in rs]
        firsts = [float(r["first_ms"]) for r in rs]
        starts = [float(r["wall_ms"]) - float(r["total_ms"]) for r in rs]
        rsss = [float(r["rss_mb"]) for r in rs]
        loads = [float(r["load1"]) for r in rs]
        out[key] = {
            "runs": len(rs),
            "wall": median(walls), "wall_min": min(walls), "wall_max": max(walls),
            "steady": median(steadies), "steady_min": min(steadies), "steady_max": max(steadies),
            "first": median(firsts), "startup": median(starts), "rss": median(rsss),
            "load": max(loads), "gc": gcs.get(key),
        }
    return out


def spread(cell, what):
    lo, hi, med = cell[what + "_min"], cell[what + "_max"], cell[what]
    return (hi - lo) / med * 100 if med else 0.0


def fmt(x, digits=1):
    return "–" if x is None else f"{x:.{digits}f}"


def report(args):
    out_dir, path = args[0], args[1]
    meta = dict(a.split("=", 1) for a in args[2:])
    rows = []
    for size in sorted(os.listdir(out_dir)) if os.path.isdir(out_dir) else []:
        results = os.path.join(out_dir, size, "results.tsv")
        if os.path.exists(results):
            rows.extend(read_rows(results))
    cells = summarize(rows)
    lines = ["# Runtime measurements", ""]
    lines.append(f"Written {datetime.datetime.now().isoformat(timespec='minutes')} by bench/runtime.sh from {out_dir}.")
    for k, v in meta.items():
        lines.append(f"- {k}: {v}")
    lines.append("")
    lines.append("Columns: wall is the process from start to exit (median of the runs, min–max); startup is wall "
                 "minus the program's own total over its K iterations; first is the first iteration, steady the "
                 "mean of the second half of the iterations; RSS is the peak resident set; GC the number of "
                 "collections in a separate logged run. Flags `teq` are those the suites' run_jvm passes to java "
                 "(-Xss512m -Xshare:auto since 2026-10-01; -XX:+UseSerialGC -XX:TieredStopAtLevel=1 as well before), `default` the JVM's own.")
    lines.append("")
    sizes = sorted({k[0] for k in cells}, key=lambda s: (s != "full", s))
    for size in sizes:
        programs = sorted({k[1] for k in cells if k[0] == size})
        lines.append(f"## Size `{size}`")
        lines.append("")
        for summary, title in ((SUMMARY, "java with run_jvm's flags"), (SUMMARY_DEFAULT, "java with default flags")):
            lines.append(f"### Summary, steady-state ms per iteration ({title}), ×best in parentheses")
            lines.append("")
            lines.append("| program | " + " | ".join(t if f in ("node",) else f"{t}" for t, f in summary) + " |")
            lines.append("|---|" + "---|" * len(summary))
            for p in programs:
                vals = [cells.get((size, p, t, f)) for t, f in summary]
                best = min((c["steady"] for c in vals if c), default=None)
                row = []
                for c in vals:
                    if c is None:
                        row.append("–")
                    else:
                        rel = c["steady"] / best if best else 0
                        row.append(f"{c['steady']:.2f} ({rel:.1f}×)")
                lines.append(f"| {p} | " + " | ".join(row) + " |")
            lines.append("")
        lines.append("### Summary, wall ms of the whole process (java with run_jvm's flags), ×best in parentheses")
        lines.append("")
        lines.append("| program | " + " | ".join(t for t, _ in SUMMARY) + " |")
        lines.append("|---|" + "---|" * len(SUMMARY))
        for p in programs:
            vals = [cells.get((size, p, t, f)) for t, f in SUMMARY]
            best = min((c["wall"] for c in vals if c), default=None)
            row = []
            for c in vals:
                row.append("–" if c is None else f"{c['wall']:.0f} ({c['wall'] / best:.1f}×)")
            lines.append(f"| {p} | " + " | ".join(row) + " |")
        lines.append("")
        for p in programs:
            lines.append(f"### {p} ({size})")
            lines.append("")
            lines.append("| toolchain | flags | runs | wall ms | startup ms | first ms | steady ms | peak RSS MB | GCs |")
            lines.append("|---|---|---|---|---|---|---|---|---|")
            for t in TOOLCHAINS:
                for f in (["node"] if t in NODE else ["teq", "default"]):
                    c = cells.get((size, p, t, f))
                    if c is None:
                        continue
                    lines.append(f"| {t} | {f} | {c['runs']} | {c['wall']:.0f} ({c['wall_min']:.0f}–{c['wall_max']:.0f}) | "
                                 f"{c['startup']:.0f} | {c['first']:.1f} | {c['steady']:.2f} ({c['steady_min']:.2f}–{c['steady_max']:.2f}) | "
                                 f"{c['rss']:.0f} | {fmt(c['gc'], 0)} |")
            lines.append("")
        walls = [spread(c, "wall") for k, c in cells.items() if k[0] == size and c["runs"] > 1]
        steadies = [spread(c, "steady") for k, c in cells.items() if k[0] == size and c["runs"] > 1]
        loads = [c["load"] for k, c in cells.items() if k[0] == size]
        if walls:
            lines.append(f"Noise at size `{size}` over {len(walls)} cells: the spread (max − min) of the runs of a cell is "
                         f"{statistics.median(walls):.1f}% of its median wall time (largest {max(walls):.1f}%) and "
                         f"{statistics.median(steadies):.1f}% of its steady-state time (largest {max(steadies):.1f}%); "
                         f"one-minute load during the runs {min(loads):.1f}–{max(loads):.1f}.")
            lines.append("")
    with open(path, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"report: {path}, {len(cells)} cells")
    return 0


def budget(args):
    results, budgets_path, tolerance = args[0], args[1], float(args[2])
    record = len(args) > 3 and args[3] == "record"
    machine = os.environ.get("MACHINE", "")
    cells = summarize(read_rows(results))
    measured = {}
    for (size, p, t, f), c in cells.items():
        if size == "quick" and (t, f) in BUDGETED:
            measured[(p, t, "wall")] = (c["wall_min"], c["wall"])
            measured[(p, t, "steady")] = (c["steady_min"], c["steady"])
    spreads = [spread(c, w) for (size, p, t, f), c in cells.items() if size == "quick" and (t, f) in BUDGETED and c["runs"] > 1 for w in ("wall", "steady")]
    noise = f"spread of a cell's runs {statistics.median(spreads):.1f}% of its median, {max(spreads):.1f}% at most" if spreads else "one run per cell"
    if record:
        with open(budgets_path, "w") as f:
            f.write(f"# machine: {machine}\n")
            f.write(f"# recorded: {datetime.date.today()}, median wall and steady-state ms at the quick size, teq-dev under node and teq-jvm under java with run_jvm's flags; {noise}\n")
            for (p, t, what), (_, med) in sorted(measured.items()):
                f.write(f"{p} {t} {what} {med:.2f}\n")
        print(f"runtime budget: recorded {len(measured)} numbers, {noise}")
        return 0
    limits = {}
    for line in open(budgets_path):
        parts = line.split()
        if len(parts) == 4 and not line.startswith("#"):
            limits[(parts[0], parts[1], parts[2])] = float(parts[3])
    failed = checked = 0
    for key, (best, med) in sorted(measured.items()):
        limit = limits.get(key)
        if limit is None:
            print(f"note: {' '.join(key)} has no budget; re-record with RUNTIME_RECORD=1")
            continue
        checked += 1
        change = (best - limit) / limit * 100
        if change > tolerance:
            print(f"FAIL {' '.join(key)}: {best:.2f} ms at best (median {med:.2f}) against a budget of {limit:.2f} ms (+{change:.0f}%)")
            failed += 1
        elif (med - limit) / limit * 100 < -tolerance:
            print(f"note: {' '.join(key)} is {med:.2f} ms against a budget of {limit:.2f} ms ({(med - limit) / limit * 100:.0f}%); lower it with RUNTIME_RECORD=1")
    print(f"runtime budget: {checked - failed} of {checked} numbers within {tolerance:.0f}% of their budget, {noise}" + (f", {failed} failed" if failed else ""))
    return 1 if failed else 0


def cpuprofile(args):
    import json
    prof = json.load(open(args[0]))
    nodes = {n["id"]: n for n in prof["nodes"]}
    self_time = {}
    for sample, delta in zip(prof["samples"], prof["timeDeltas"]):
        frame = nodes[sample]["callFrame"]
        key = f"{frame['functionName'] or '(anonymous)'} ({os.path.basename(frame['url']) or 'native'}:{frame['lineNumber'] + 1})"
        self_time[key] = self_time.get(key, 0) + delta
    total = sum(self_time.values()) or 1
    for key, t in sorted(self_time.items(), key=lambda kv: -kv[1])[:int(args[1]) if len(args) > 1 else 10]:
        print(f"{t / total * 100:5.1f}%  {key}")
    return 0


def jfr(args):
    text = subprocess.run(["jfr", "print", "--events", "jdk.ExecutionSample", args[0]], capture_output=True, text=True, check=True).stdout
    counts = {}
    for m in re.finditer(r"stackTrace = \[\s*\n\s*([^\n]+?)\s*(?:line: \d+)?\s*\n", text):
        counts[m.group(1)] = counts.get(m.group(1), 0) + 1
    total = sum(counts.values()) or 1
    print(f"{total} samples")
    for frame, n in sorted(counts.items(), key=lambda kv: -kv[1])[:int(args[1]) if len(args) > 1 else 10]:
        print(f"{n / total * 100:5.1f}%  {frame}")
    return 0


if __name__ == "__main__":
    commands = {"measure": measure, "gc": gc, "report": report, "budget": budget, "cpuprofile": cpuprofile, "jfr": jfr}
    if len(sys.argv) < 2 or sys.argv[1] not in commands:
        print(__doc__)
        sys.exit(2)
    sys.exit(commands[sys.argv[1]](sys.argv[2:]))
