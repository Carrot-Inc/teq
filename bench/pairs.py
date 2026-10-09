#!/usr/bin/env python3
"""Two teq binaries compared on one machine by balanced pairs: the gate's budget and runtime lines
on a remote machine (tests/gate.sh, docs/DEVELOPING.md "The timing lines on a machine").

  budget <master teq> <head teq> [options]
      the type phase of `teq compiler check --time` on every program of bench/programs.sh, the rows of
      bench/budgets.txt's type phase, every one present
  runtime <master teq> <head teq> [options]
      the quick size of bench/runtime.sh built by each binary (teq-dev under node, teq-jvm linked
      against scala-library and under java beside its jar, with bench/runtime.sh's flags), wall and steady-state time, the rows of
      bench/runtime-budgets.txt, every output checked against tests/cases; node and java are
      checked first against bench/runtime-machine.txt
  judge <runs.jsonl> [options]
      the verdicts of recorded runs again, under other settings
  advance <runs.jsonl>
      the line of bench/reference.txt that advances the reference to the head of a comparison of the
      sentinel's rows, the old reference as master: each row's ratio, the median of its cycles' means
  floors <runs.jsonl>...
      per class of rows (type; a runtime and a metric), the spread of the medians across every window of N
      cycles, the band rule's floor of the standard error
  calibrate <runs.jsonl>... [--tolerance pct] [--band k] [--limit x] [--trim n]
      for N of 3, 5 and 7 cycles, over every window of N consecutive cycles of runs of one binary
      against itself: the dispersion of the cycles' means and how often a row fails or is
      inconclusive

The schedule: each program (a cell of the runtime line, one program on one runtime) starts with an
unmeasured run of each binary, then runs in cycles, a cycle being a master-head pair followed by a
head-master pair (the program's starting side first, the side alternating from one program to the
next); the cycles go round every program in turn, so that a burst of load meets one cycle of the
programs it falls on. Per row, each cycle gives the head's faster run over master's faster run, and
the row is judged on the median over the cycles and its band (see RULE); the cells with an
inconclusive row are measured once more, and a row inconclusive again leaves the line
inconclusive. Exit 0 within, 1 a row over or a run failed, 2 not run, 3
inconclusive.

Options: --out <dir> (default out/pairs/<line>), --cycles N, --tolerance <pct>, --ceilings <file>
(per-row ceilings over the reference, bench/reference.txt), --limit <x>, --programs <a,b,..>,
--start master|head, --reverse, --head-env NAME=VALUE (the head's runs and builds only),
--no-rerun, --label <name>, --unpinned (the runtime line on a machine bench/runtime-machine.txt
does not name, for runs by hand).
"""
import datetime
import json
import math
import os
import platform
import re
import shutil
import statistics
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "bench"))
import runtime as harness  # noqa: E402

CYCLES = 7
TOLERANCE = {"budget": 8.0, "runtime": 15.0}
LIMIT = {"budget": 1.30, "runtime": 1.30}
# How a row's cycles are judged. "band": the median of the cycle means and its standard error, from their
# median absolute deviation in logarithms; over when the median less BAND standard errors exceeds the
# tolerance, within when the median plus them does not, inconclusive between. "range": inconclusive when the
# largest cycle mean over the smallest (TRIM of them left out at each end) exceeds the limit, else over when the
# median exceeds the tolerance.
RULE = "band"
CYCLE = "faster"
BAND = 2.0
TRIM = 0
# Under the band rule the standard error is at least the spread of the medians a row class showed across the
# windows of the calibration's sequences without pressure (bench/pairs.py floors), as a fraction in logarithms,
# so that seven cycles that happen to agree closely cannot fail a row the medians' own spread would not.
# From the sequences without pressure of 2026-10-01 on two remote machines (docs/DEVELOPING.md, "The timing
# lines on a machine"): type from A1, B1, A4, B4, the runtime classes from A1n, B1n, A4n, B4n.
FLOOR = {"type": 0.0096, "teq-dev steady": 0.0115, "teq-dev wall": 0.0130, "teq-jvm steady": 0.0179, "teq-jvm wall": 0.0174}


def floor_class(cell, metric):
    return f"{cell.split('.')[1]} {metric}" if "." in cell else metric
SIDES = ("master", "head")
CHECK_TIMEOUT = 60
BUILD_TIMEOUT = 300
RUN_TIMEOUT = 120
RUNTIMES = (("teq-dev", "node"), ("teq-jvm", "java"))


def fail_setup(message):
    print(message)
    sys.exit(2)


def parse_options(argv):
    opts = {"out": None, "cycles": None, "tolerance": None, "ceilings": None, "limit": None, "programs": None, "trim": None, "band": None,
            "start": "master", "reverse": False, "head_env": {}, "rerun": True, "label": None, "unpinned": False}
    rest = []
    i = 0
    while i < len(argv):
        a = argv[i]
        value = argv[i + 1] if i + 1 < len(argv) else None
        if a in ("--out", "--cycles", "--tolerance", "--ceilings", "--limit", "--programs", "--start", "--head-env", "--label", "--trim", "--band"):
            if value is None:
                fail_setup(f"pairs: {a} takes a value")
            if a == "--head-env":
                name, _, val = value.partition("=")
                opts["head_env"][name] = val
            elif a in ("--cycles", "--trim"):
                opts[a[2:]] = int(value)
            elif a in ("--tolerance", "--limit", "--band"):
                opts[a[2:]] = float(value)
            elif a == "--programs":
                opts["programs"] = [p for p in value.replace(",", " ").split() if p]
            else:
                opts[a[2:]] = value
            i += 2
            continue
        if a == "--reverse":
            opts["reverse"] = True
        elif a == "--no-rerun":
            opts["rerun"] = False
        elif a == "--unpinned":
            opts["unpinned"] = True
        elif a.startswith("--"):
            fail_setup(f"pairs: no option {a}")
        else:
            rest.append(a)
        i += 1
    if opts["start"] not in SIDES:
        fail_setup("pairs: --start is master or head")
    return opts, rest


def machine_identity():
    cpu = ""
    try:
        cpu = next((l.split(":", 1)[1].strip() for l in open("/proc/cpuinfo") if l.startswith("model name")), "")
    except OSError:
        cpu = subprocess.run(["sysctl", "-n", "machdep.cpu.brand_string"], capture_output=True, text=True).stdout.strip()
    cpuset = ""
    try:
        cpuset = open("/sys/fs/cgroup/cpuset.cpus.effective").read().strip()
    except OSError:
        pass
    return f"{platform.node()} {platform.machine()} {cpu} {os.cpu_count()} cores" + (f", cpuset {cpuset}" if cpuset else "")


def load1():
    """The one-minute load average, on a remote machine the host's, which every machine there shares."""
    try:
        return round(os.getloadavg()[0], 2)
    except OSError:
        return 0.0


def own_cpu():
    """The CPU this machine's own processes use, in percent of a thread, as pool.sh reads it."""
    out = subprocess.run(["ps", "-eo", "pcpu"], capture_output=True, text=True, timeout=30).stdout.split()[1:]
    return round(sum(float(x) for x in out if x.replace(".", "", 1).isdigit()))


def version_of(teq):
    out = subprocess.run([teq, "--version"], capture_output=True, text=True, timeout=30)
    return out.stdout.strip() or f"(no version: exit {out.returncode})"


# Runs: one JSON object a line, {attempt, cycle, pair, side, cell, at, values} where cycle -1 is the
# warm-up and pair is 1 or 2.

def schedule(cells, cycles, start, reverse):
    order = list(reversed(cells)) if reverse else list(cells)
    first = SIDES.index(start)
    for c in range(cycles):
        for i, cell in enumerate(order):
            s = SIDES[(first + i) % 2]
            o = SIDES[(first + i + 1) % 2]
            if c == 0:
                yield (-1, 0, s, cell)
                yield (-1, 0, o, cell)
            yield (c, 1, s, cell)
            yield (c, 1, o, cell)
            yield (c, 2, o, cell)
            yield (c, 2, s, cell)


def measure_attempt(attempt, cells, opts, run_one, runs_file, started):
    errors = []
    with open(runs_file, "a") as out:
        for cycle, pair, side, cell in schedule(cells, opts["cycles"], opts["start"], opts["reverse"]):
            at = round(time.monotonic() - started, 3)
            values, error = run_one(side, cell)
            if error:
                errors.append(f"{cell} {side}: {error}")
                return errors
            out.write(json.dumps({"attempt": attempt, "cycle": cycle, "pair": pair, "side": side, "cell": cell,
                                  "at": at, "t": round(time.time(), 3), "load": load1(), "values": values}) + "\n")
    return errors


def cycle_means(runs, cell, metric, statistic=None):
    """Per complete cycle the head's time over master's: under CYCLE "pairs" the geometric mean of the two pairs'
    ratios, under "faster" each side's faster run of the cycle over the other's, since a neighbour's load only
    ever adds time to a run."""
    by = {}
    for r in runs:
        if r["cell"] == cell and r["cycle"] >= 0 and metric in r["values"]:
            by[(r["cycle"], r["pair"], r["side"])] = r["values"][metric]
    means = []
    for c in sorted({k[0] for k in by}):
        sides = {side: [by.get((c, pair, side)) for pair in (1, 2)] for side in SIDES}
        if None in sides["master"] + sides["head"]:
            continue
        if (statistic or CYCLE) == "faster":
            m, h = min(sides["master"]), min(sides["head"])
            means.append(h / m if m > 0 else (1.0 if h == 0 else math.inf))
        else:
            ratios = [h / m if m > 0 else (1.0 if h == 0 else math.inf) for m, h in zip(sides["master"], sides["head"])]
            means.append(math.sqrt(ratios[0] * ratios[1]))
    return means


def dispersion_of(means, trim=None):
    kept = sorted(means)
    t = TRIM if trim is None else trim
    if t and len(kept) > 2 * t:
        kept = kept[t:len(kept) - t]
    return kept[-1] / kept[0] if kept[0] > 0 else math.inf


def band_of(means):
    logs = [math.log(m) for m in means]
    med = statistics.median(logs)
    mad = statistics.median(abs(x - med) for x in logs)
    return math.exp(med), 1.253 * 1.4826 * mad / math.sqrt(len(logs))


def verdict(means, tolerance, limit, rule=None, band=None, floor=0.0):
    """(verdict, the median, the spread: the standard error under the band rule, the dispersion under the range rule)"""
    if not means:
        return "missing", None, None
    if (rule or RULE) == "band":
        k = band or BAND
        med, se = band_of(means)
        se = max(se, floor)
        t = math.log(1 + tolerance / 100)
        if math.log(med) - k * se > t:
            return "over", med, se
        if math.log(med) + k * se <= t:
            return "within", med, se
        return "inconclusive", med, se
    med = statistics.median(means)
    dispersion = dispersion_of(means)
    if dispersion > limit:
        return "inconclusive", med, dispersion
    if (med - 1) * 100 > tolerance:
        return "over", med, dispersion
    return "within", med, dispersion


def rule_text(limit):
    return f"{BAND:g} standard errors" if RULE == "band" else f"dispersion limit {limit}" + (f", trimmed by {TRIM}" if TRIM else "")


def rows_of(runs):
    rows = []
    for r in runs:
        for metric in r["values"]:
            key = (r["cell"], metric)
            if key not in rows:
                rows.append(key)
    return rows


def judge_rows(runs, rows, tolerances, limit):
    judged = {}
    for cell, metric in rows:
        means = cycle_means(runs, cell, metric)
        judged[(cell, metric)] = verdict(means, tolerances(cell, metric), limit, floor=FLOOR.get(floor_class(cell, metric), 0.0)) + (means,)
    return judged


def report_row(cell, metric, judged, tolerance, attempt):
    v, med, spread, means = judged
    mark = {"within": "ok", "over": "FAIL", "inconclusive": "INCONCLUSIVE", "missing": "FAIL"}[v]
    if v == "missing":
        return f"{mark} {cell} {metric}: not measured"
    cycles = " ".join(f"{(m - 1) * 100:+.1f}" for m in means)
    if RULE == "band":
        low, high = (med * math.exp(-BAND * spread) - 1) * 100, (med * math.exp(BAND * spread) - 1) * 100
        spread = f"band {low:+.1f} to {high:+.1f}% (standard error {spread * 100:.1f}%)"
    else:
        spread = f"dispersion {spread:.3f}"
    return (f"{mark} {cell} {metric}: {(med - 1) * 100:+.1f}% (tolerance {tolerance:.1f}%), {spread}, "
            f"attempt {attempt}, cycles {cycles}")


def read_ceilings(path):
    """Per row, the ratio over the reference a head may reach (bench/reference.txt): the ceiling of the
    last `reference` line, divided by each row's ratio at every `advance` since."""
    ceilings = {}
    for line in open(path):
        parts = line.split("#", 1)[0].split()
        if parts and parts[0] == "reference":
            ceilings = {"*": float(parts[parts.index("ceiling") + 1])}
        elif parts and parts[0] == "advance":
            for p in parts[3:]:
                row, ratio = p.split("=")
                ceilings[row] = ceilings.get(row, ceilings["*"]) / float(ratio)
    return ceilings


def reference_of(path):
    revision = None
    for line in open(path):
        parts = line.split("#", 1)[0].split()
        if parts and parts[0] in ("reference", "advance"):
            revision = parts[1]
    return revision


def program_list():
    script = ('work=out/budget; . bench/programs.sh > /dev/null || exit 1; '
              '[ -z "$left_out" ] || { printf "%s" "$left_out"; exit 3; }; '
              'for p in $programs; do printf "%s\\t%s\\n" "$p" "$(program_args "$p")"; done')
    out = subprocess.run(["bash", "-c", script], cwd=ROOT, capture_output=True, text=True, timeout=600)
    if out.returncode == 3:
        fail_setup("pairs: the suite is not whole, programs left out for want of their jars:\n" + out.stdout)
    if out.returncode != 0:
        fail_setup("pairs: bench/programs.sh failed: " + (out.stderr or out.stdout)[-500:])
    return [tuple(l.split("\t", 1)) for l in out.stdout.splitlines() if l.strip()]


def frozen_rows(path, width):
    rows = []
    for line in open(path):
        parts = line.split()
        if line.startswith("#") or len(parts) != width:
            continue
        rows.append(tuple(parts[:-1]))
    return rows


def phases_of(text):
    out = subprocess.run(["bash", "-c", ". bench/phases.sh && phases"], cwd=ROOT, input=text, capture_output=True,
                         text=True, timeout=30)
    if out.returncode != 0:
        return None
    fields = out.stdout.split()
    values = dict(zip(fields[0::2], fields[1::2]))
    try:
        return {k: float(values[k]) for k in ("parse", "type", "total")}
    except (KeyError, ValueError):
        return None


_compiler_words = {}


def compiler_words(binary):
    """The words before the compiler's verb for a binary: `compiler` under the project verbs' spelling (`<binary>
    compiler --help` exits 0), none under the spelling before it (the usage, exit 2); probed once per path. Goes
    when bench/reference.txt advances past the first release carrying the project verbs at the top level."""
    if binary not in _compiler_words:
        probe = subprocess.run([binary, "compiler", "--help"], capture_output=True)
        _compiler_words[binary] = ["compiler"] if probe.returncode == 0 else []
    return _compiler_words[binary]


def side_env(side, opts, out_dir):
    env = dict(os.environ)
    env["TEQ_CACHE_DIR"] = os.path.join(out_dir, "cache", side)
    if side == "head":
        env.update(opts["head_env"])
    return env


def budget_line(teqs, opts):
    programs = dict(program_list())
    if opts["programs"]:
        rows = [(p, "type") for p in opts["programs"]]
    else:
        rows = [r for r in frozen_rows(os.path.join(ROOT, "bench/budgets.txt"), 3) if r[1] == "type"]
    missing = [p for p, _ in rows if p not in programs]
    unbudgeted = [] if opts["programs"] else [p for p in programs if (p, "type") not in rows]
    if missing or unbudgeted:
        fail_setup("pairs: the rows and the programs differ:" + "".join(f" {p} (no program)" for p in missing)
                   + "".join(f" {p} (no row in bench/budgets.txt)" for p in unbudgeted))
    cells = [p for p, _ in rows]
    out_dir = opts["out"]

    def run_one(side, cell):
        cmd = [teqs[side], *compiler_words(teqs[side]), "check"] + programs[cell].split() + ["--time"]
        try:
            p = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=CHECK_TIMEOUT,
                               env=side_env(side, opts, out_dir))
        except subprocess.TimeoutExpired:
            return None, f"teq compiler check timed out after {CHECK_TIMEOUT} s"
        if p.returncode != 0:
            return None, f"teq compiler check exited {p.returncode}: {(p.stdout + p.stderr).strip()[-300:]}"
        values = phases_of(p.stdout + p.stderr)
        if values is None:
            return None, "no phases in the output of teq compiler check --time"
        return {"type": values["type"]}, None

    return cells, [(c, "type") for c in cells], run_one


def java_flags():
    text = open(os.path.join(ROOT, "bench/runtime.sh")).read()
    m = re.search(r'^teq_java_flags="([^"]*)"$', text, re.M)
    if not m:
        fail_setup("pairs: no teq_java_flags line in bench/runtime.sh")
    return m.group(1).split()


def scala_library_jar():
    """bench/runtime.sh's pinned scala-library, which teq-jvm links against: in $COURSIER_CACHE or either
    conventional coursier cache, as bench/runtime.sh looks."""
    text = open(os.path.join(ROOT, "bench/runtime.sh")).read()
    m = re.search(r'^scala_version=(\S+)$', text, re.M)
    if not m:
        fail_setup("pairs: no scala_version line in bench/runtime.sh")
    version = m.group(1)
    home = os.path.expanduser("~")
    caches = ([os.environ["COURSIER_CACHE"]] if os.environ.get("COURSIER_CACHE") else []) + \
        [os.path.join(home, "Library/Caches/Coursier/v1"), os.path.join(home, ".cache/coursier/v1")]
    for cache in caches:
        jar = os.path.join(cache, f"https/repo1.maven.org/maven2/org/scala-lang/scala-library/{version}/scala-library-{version}.jar")
        if os.path.isfile(jar):
            return jar
    fail_setup(f"pairs: scala-library {version} is in no coursier cache ({', '.join(caches)})")


def quick_sizes():
    text = open(os.path.join(ROOT, "bench/runtime.sh")).read()
    block = re.search(r'^sizes="\n(.*?)^"$', text, re.M | re.S)
    if not block:
        fail_setup("pairs: no sizes table in bench/runtime.sh")
    return {f[0]: (f[1], f[2]) for f in (l.split() for l in block.group(1).splitlines()) if len(f) == 5}


def tool_identity(name):
    path = shutil.which(name)
    if not path:
        return None
    real = os.path.realpath(path)
    if name == "node":
        version = subprocess.run([real, "--version"], capture_output=True, text=True, timeout=30).stdout.strip()
    else:
        version = subprocess.run([real, "-version"], capture_output=True, text=True, timeout=30).stderr.splitlines()[0]
    return f"{name} {real} {version}"


def check_runtimes(opts):
    pinned = os.path.join(ROOT, "bench/runtime-machine.txt")
    have = [tool_identity("node"), tool_identity("java")]
    if None in have:
        fail_setup("pairs: no node or no java on this machine")
    want = [l.strip() for l in open(pinned) if l.strip() and not l.startswith("#")] if os.path.exists(pinned) else []
    if have != want:
        message = "pairs: node and java here are not bench/runtime-machine.txt's:\n  here: " + "\n        ".join(have) + \
                  "\n  recorded: " + "\n            ".join(want)
        if not opts["unpinned"]:
            fail_setup(message)
        print(message.replace("pairs:", "pairs: unpinned,", 1))
    return have


def runtime_line(teqs, opts):
    tools = check_runtimes(opts)
    sizes = quick_sizes()
    # The programs are bench/runtime's, which bench/runtime.sh's sizes table has to list alike; every one of them
    # has its four rows in bench/runtime-budgets.txt, whatever else that file says.
    inventory = sorted(f[:-len(".scala")] for f in os.listdir(os.path.join(ROOT, "bench/runtime")) if f.endswith(".scala"))
    if inventory != sorted(sizes):
        fail_setup(f"pairs: bench/runtime holds {' '.join(inventory)} and bench/runtime.sh's sizes {' '.join(sorted(sizes))}")
    frozen = frozen_rows(os.path.join(ROOT, "bench/runtime-budgets.txt"), 4)
    every = {(p, t, m) for p in inventory for t, _ in RUNTIMES for m in ("steady", "wall")}
    if set(frozen) != every:
        fail_setup("pairs: bench/runtime-budgets.txt's rows are not every program's four: "
                   + " ".join(sorted(f"{'-' if r in every else '+'}{' '.join(r)}" for r in set(frozen) ^ every)))
    programs = inventory
    if opts["programs"]:
        programs = [p for p in programs if p in opts["programs"]]
    out_dir = opts["out"]
    flags = java_flags()
    # Both binaries link teq-jvm against the one jar, the head's only JVM mode and master's under
    # --std=scala-library, which a master from before the lean JVM mode's retirement needs.
    scala_library = scala_library_jar()
    src_dir = os.path.join(out_dir, "sources")
    os.makedirs(src_dir, exist_ok=True)
    for p in programs:
        rounds, k = sizes[p]
        text = open(os.path.join(ROOT, f"bench/runtime/{p}.scala")).read()
        default = re.search(r'^  val defaults = "(\d+) ', text, re.M)
        if not default or default.group(1) != rounds:
            fail_setup(f"pairs: bench/runtime/{p}.scala's default rounds are not its quick size's {rounds}, "
                       f"which tests/cases/bench_{p}.expected's checksum is of")
        quick, n = re.subn(r'^  val defaults = ".*"$', f'  val defaults = "{rounds} {k} 1"', text, flags=re.M)
        if n != 1:
            fail_setup(f"pairs: no defaults line in bench/runtime/{p}.scala")
        with open(os.path.join(src_dir, f"{p}.scala"), "w") as f:
            f.write(quick)
    # The artifacts of each binary in a directory of its own, built afresh: nothing is reused by date.
    artifacts = {}
    for side in SIDES:
        side_dir = os.path.join(out_dir, "artifacts", side)
        shutil.rmtree(side_dir, ignore_errors=True)
        os.makedirs(side_dir)
        env = side_env(side, opts, out_dir)
        for p in programs:
            for toolchain, _ in RUNTIMES:
                artifact = os.path.join(side_dir, f"{p}.{toolchain}" + (".jar" if toolchain == "teq-jvm" else ".js"))
                cmd = [teqs[side], *compiler_words(teqs[side]), "build", os.path.join(src_dir, f"{p}.scala"), "-o", artifact]
                if toolchain == "teq-jvm":
                    cmd[-2:-2] = ["--target", "jvm", "--std=scala-library", "--classpath", scala_library]
                try:
                    b = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=BUILD_TIMEOUT, env=env)
                except subprocess.TimeoutExpired:
                    fail_setup(f"pairs: {side}'s build of {p} ({toolchain}) timed out")
                if b.returncode != 0:
                    print(f"FAIL {p} {toolchain}: {side}'s build exited {b.returncode}: {(b.stdout + b.stderr).strip()[-300:]}")
                    sys.exit(1)
                artifacts[(side, p, toolchain)] = artifact
    cells = [f"{p}.{t}" for p in programs for t, _ in RUNTIMES]
    rows = [(f"{p}.{t}", m) for p in programs for t, _ in RUNTIMES for m in ("steady", "wall")]
    wanted = {(f"{p}.{t}", m) for p, t, m in frozen if p in programs}
    if set(rows) != wanted:
        fail_setup("pairs: bench/runtime-budgets.txt's rows are not the line's: " + " ".join(sorted(f"{c} {m}" for c, m in set(rows) ^ wanted)))
    expected = {}
    for p in programs:
        path = os.path.join(ROOT, f"tests/cases/bench_{p}.expected")
        if not os.path.exists(path):
            fail_setup(f"pairs: no {path}")
        expected[p] = open(path, "rb").read()
    scratch = os.path.join(out_dir, "run")
    os.makedirs(scratch, exist_ok=True)

    def run_one(side, cell):
        p, toolchain = cell.split(".")
        artifact = artifacts[(side, p, toolchain)]
        cmd = ["node", artifact] if toolchain == "teq-dev" else ["java"] + flags + ["-cp", artifact + os.pathsep + scala_library, "TeqMain"]
        out_path, err_path = os.path.join(scratch, "out"), os.path.join(scratch, "err")
        wall, code, _ = harness.run_once(cmd, RUN_TIMEOUT, out_path, err_path)
        if code != 0:
            return None, f"exit {code}: {open(err_path, errors='replace').read()[-300:]}"
        if open(out_path, "rb").read() != expected[p]:
            return None, f"the output differs from tests/cases/bench_{p}.expected"
        times = harness.bench_line(err_path)
        if times is None:
            return None, "no bench line on stderr"
        return {"wall": round(wall, 2), "steady": times[2]}, None

    return cells, rows, run_one, tools


def run_line(kind, master, head, opts):
    label = opts["label"] or kind
    opts["out"] = opts["out"] or os.path.join(ROOT, "out", "pairs", label)
    opts["cycles"] = opts["cycles"] or CYCLES
    limit = opts["limit"] or LIMIT[kind]
    base_tolerance = opts["tolerance"] if opts["tolerance"] is not None else TOLERANCE[kind]
    ceilings = read_ceilings(opts["ceilings"]) if opts["ceilings"] else None

    def tolerances(cell, metric):
        if ceilings is None:
            return base_tolerance
        return (ceilings.get(cell, ceilings["*"]) - 1) * 100

    teqs = {"master": os.path.abspath(master), "head": os.path.abspath(head)}
    for side, teq in teqs.items():
        if not os.access(teq, os.X_OK):
            fail_setup(f"pairs: {side}'s binary {teq} is missing")
    os.makedirs(opts["out"], exist_ok=True)
    runs_file = os.path.join(opts["out"], "runs.jsonl")
    if os.path.exists(runs_file):
        os.remove(runs_file)
    tools = None
    if kind == "budget":
        cells, rows, run_one = budget_line(teqs, opts)
    else:
        cells, rows, run_one, tools = runtime_line(teqs, opts)
    header = {"line": label, "kind": kind, "master": version_of(teqs["master"]), "head": version_of(teqs["head"]),
              "machine": machine_identity(), "cycles": opts["cycles"], "limit": limit, "tolerance": base_tolerance,
              "ceilings": opts["ceilings"], "start": opts["start"], "reverse": opts["reverse"], "rule": RULE, "band": BAND, "trim": TRIM,
              "head_env": opts["head_env"], "tools": tools, "when": datetime.datetime.now().isoformat(timespec="seconds")}
    with open(os.path.join(opts["out"], "line.json"), "w") as f:
        json.dump(header, f, indent=1)
    print(f"{label}: head {header['head']} against master {header['master']} on {header['machine']}; "
          f"{len(rows)} rows, {opts['cycles']} cycles, {rule_text(limit)}" + (f", head env {opts['head_env']}" if opts["head_env"] else ""))
    started = time.monotonic()
    load_start, cpu_start = load1(), own_cpu()
    errors = measure_attempt(1, cells, opts, run_one, runs_file, started)
    if errors:
        print(f"FAIL {errors[0]}")
        print(f"{label}: a run failed, nothing judged")
        return 1
    runs = [json.loads(l) for l in open(runs_file)]
    judged = judge_rows(runs, rows, tolerances, limit)
    attempt_of = {row: 1 for row in rows}
    again = sorted({cell for (cell, metric), j in judged.items() if j[0] == "inconclusive"}, key=cells.index)
    if again and opts["rerun"]:
        print(f"{label}: {len(again)} cells inconclusive, measured again: {' '.join(again)}")
        errors = measure_attempt(2, again, opts, run_one, runs_file, started)
        if errors:
            print(f"FAIL {errors[0]}")
            print(f"{label}: a run failed, nothing judged")
            return 1
        second = [json.loads(l) for l in open(runs_file) if json.loads(l)["attempt"] == 2]
        for row, j in judge_rows(second, [r for r in rows if r[0] in again and judged[r][0] == "inconclusive"], tolerances, limit).items():
            judged[row] = j
            attempt_of[row] = 2
    for row in rows:
        print(report_row(row[0], row[1], judged[row], tolerances(*row), attempt_of[row]))
    counts = {v: sum(1 for j in judged.values() if j[0] == v) for v in ("within", "over", "inconclusive", "missing")}
    seconds = time.monotonic() - started
    tol = "the reference's ceilings" if ceilings else f"{base_tolerance:.0f}%"
    print(f"{label}: {counts['within']} of {len(rows)} rows within {tol}, {counts['over'] + counts['missing']} over, "
          f"{counts['inconclusive']} inconclusive; {opts['cycles']} cycles, {rule_text(limit)}, {seconds:.0f} s, "
          f"host load {load_start} at the start and {load1()} at the end, the machine's own CPU {cpu_start}% and {own_cpu()}%")
    if counts["over"] or counts["missing"]:
        return 1
    return 3 if counts["inconclusive"] else 0


def header_of(path):
    """The line.json beside a runs.jsonl, or the gate's copy of it, <line>.line.json beside <line>.runs.jsonl."""
    beside = os.path.join(os.path.dirname(path), "line.json")
    return json.load(open(beside if path.endswith("/runs.jsonl") or not path.endswith(".runs.jsonl") else path[:-len(".runs.jsonl")] + ".line.json"))


def judge_file(path, opts):
    header = header_of(path)
    kind = header["kind"]
    runs = [json.loads(l) for l in open(path)]
    cycles = opts["cycles"]
    first = [r for r in runs if r["attempt"] == 1 and (cycles is None or r["cycle"] < cycles)]
    limit = opts["limit"] or header["limit"]
    tolerance = opts["tolerance"] if opts["tolerance"] is not None else header["tolerance"]
    rows = rows_of(first)
    judged = judge_rows(first, rows, lambda c, m: tolerance, limit)
    for row in rows:
        print(report_row(row[0], row[1], judged[row], tolerance, 1))
    counts = {v: sum(1 for j in judged.values() if j[0] == v) for v in ("within", "over", "inconclusive")}
    print(f"{header['line']} ({kind}): {counts['within']} within {tolerance}%, {counts['over']} over, "
          f"{counts['inconclusive']} inconclusive, {rule_text(limit)}")
    return 0


def row_class(kind, runs, row):
    cell, metric = row
    if kind == "runtime":
        return f"{cell.split('.')[1]} {metric}"
    ms = statistics.median(r["values"][metric] for r in runs if r["cell"] == cell and r["side"] == "master")
    return "type under 10 ms" if ms < 10 else "type 10 to 100 ms" if ms < 100 else "type over 100 ms"


def calibrate(paths, opts):
    """Runs of one binary against itself (or two builds of one commit): every window of N consecutive
    cycles of a sequence stands for a gate run of N cycles. Per class of rows and N, the windows' median
    change at quantiles, then the verdicts a gate would give them: under the band rule for 1.5 to 3
    standard errors, under the range rule for limits of 1.2 to 1.6 (--trim the cycles left out at each
    end); --band or --limit names one."""
    per = {}
    floors = {}
    for path in paths:
        header = header_of(path)
        runs = [json.loads(l) for l in open(path) if json.loads(l)["attempt"] == 1]
        for row in rows_of(runs):
            means = cycle_means(runs, *row)
            klass = row_class(header["kind"], runs, row)
            for n in (3, 5, 7):
                for s in range(0, len(means) - n + 1):
                    per.setdefault((header["kind"], klass, n), []).append(means[s:s + n])
                    floors.setdefault((header["kind"], klass, n), []).append(FLOOR.get(floor_class(*row), 0.0))
    q = lambda xs, f: xs[min(len(xs) - 1, int(f * len(xs)))]
    settings = [("band", k, None) for k in ([opts["band"]] if opts["band"] else [1.5, 2.0, 2.5, 3.0])]
    settings += [("range", None, l) for l in ([opts["limit"]] if opts["limit"] else [1.2, 1.3, 1.4, 1.5, 1.6])]
    for (kind, klass, n), windows in sorted(per.items()):
        tolerance = opts["tolerance"] if opts["tolerance"] is not None else TOLERANCE[kind]
        meds = sorted((statistics.median(w) - 1) * 100 for w in windows)
        print(f"{kind}, {klass}, N={n}: {len(windows)} windows; median change 1% {q(meds, .01):+.2f}%, "
              f"99% {q(meds, .99):+.2f}%, largest {meds[-1]:+.2f}%; over {tolerance}%: {sum(1 for m in meds if m > tolerance)}")
        for rule, k, limit in settings:
            if rule == "range":
                global TRIM
                saved, TRIM = TRIM, opts["trim"] or 0
            vs = [verdict(w, tolerance, limit, rule, k, floor)[0] for w, floor in zip(windows, floors[(kind, klass, n)])]
            if rule == "range":
                TRIM = saved
            what = f"band {k}" if rule == "band" else f"range {limit}" + (f" trim {opts['trim']}" if opts["trim"] else "")
            print(f"    {what}: {vs.count('inconclusive')} inconclusive, {vs.count('over')} over")
    return 0


def floors(paths):
    """Per floor class, the standard deviation of the log medians of every window of CYCLES consecutive cycles
    of the sequences given, the band rule's floor."""
    medians = {}
    for path in paths:
        runs = [json.loads(l) for l in open(path) if json.loads(l)["attempt"] == 1]
        for row in rows_of(runs):
            means = cycle_means(runs, *row)
            for s in range(0, len(means) - CYCLES + 1):
                medians.setdefault(floor_class(*row), []).append(math.log(statistics.median(means[s:s + CYCLES])))
    for klass, xs in sorted(medians.items()):
        print(f"{klass}: {len(xs)} windows of {CYCLES} cycles, the medians' standard deviation {statistics.stdev(xs) * 100:.2f}%"
              f" (largest {max(abs(x) for x in xs) * 100:.2f}%)")
    return 0


def advance_line(path):
    """Printed only from a whole comparison: every row bench/reference.txt names with all its cycles, the old
    side the reference the file names, the new side a plain build of a named revision."""
    header = header_of(path)
    reference_file = os.path.join(ROOT, "bench/reference.txt")
    wanted = next((l.split()[1:] for l in open(reference_file) if l.startswith("rows ")), [])
    old, new = header["master"].split(), header["head"].split()
    current = reference_of(reference_file)
    if len(old) != 3 or not current or not (old[2].startswith(current) or current.startswith(old[2])):
        fail_setup(f"pairs: the old side is {header['master']}, not the reference {current} that bench/reference.txt names")
    if len(new) != 3:
        fail_setup(f"pairs: the new side is {header['head']}, not a plain build of a named revision")
    runs = [json.loads(l) for l in open(path) if json.loads(l)["attempt"] == 1]
    ratios = []
    for cell in wanted:
        means = cycle_means(runs, cell, "type")
        if len(means) < header["cycles"]:
            fail_setup(f"pairs: {cell} has {len(means)} complete cycles of {header['cycles']}; nothing advanced")
        ratios.append(f"{cell}={statistics.median(means):.4f}")
    if not wanted:
        fail_setup("pairs: bench/reference.txt names no rows")
    print(f"advance {new[2]} {datetime.date.today()} " + " ".join(ratios))
    return 0


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in ("budget", "runtime", "judge", "calibrate", "advance", "floors"):
        print(__doc__)
        return 2
    command = sys.argv[1]
    opts, rest = parse_options(sys.argv[2:])
    if command in ("budget", "runtime"):
        if len(rest) != 2:
            fail_setup(f"usage: bench/pairs.py {command} <master teq> <head teq> [options]")
        return run_line(command, rest[0], rest[1], opts)
    if command == "judge":
        return judge_file(rest[0], opts)
    if command == "advance":
        return advance_line(rest[0])
    if command == "floors":
        return floors(rest)
    return calibrate(rest, opts)


if __name__ == "__main__":
    sys.exit(main())
