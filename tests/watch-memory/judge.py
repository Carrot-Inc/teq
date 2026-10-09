# The judge of tests/watch-memory.sh: a session's line of the driver against its budget.
#
#   judge.py lines <file>    judges the lines of a run, or records them as budgets (RECORD)
#   judge.py accepts <session> <edits>
#                            whether a session of that many edits can be judged: it has to have
#                            the edits its budget was recorded at, whose memory at the end the
#                            budget holds, or two hundred and more, whose memory at the end is
#                            held against the memory after the hundredth; a peak row, none. Exits
#                            2 for a session whose budget is another platform's alone (`only`),
#                            which is not run here, and says so
#   judge.py baseline <session>
#                            the session a peak row's ceiling is a ratio of, if any
#   judge.py failing <file>  the sessions of a run's lines whose memory fails their budgets, one a
#                            line with its first failure: which of them to run again (not one that
#                            has no row to be held to, which a run again would not give it)
#   judge.py self-test       judges made-up lines and records made-up budgets, which says
#                            whether the judge can fail and the recording keeps what it does not
#                            record
#
# A session's budget is a row `<platform> <session> edits <n> growth_kb_per_edit <kb>
# memory_end_mb <mb> metric <m>`, the retained side, in the metric its line names (the driver's
# header: `Rss+Swap` and the language server's own `Anonymous+Swap` on Linux, `phys_footprint` on
# macOS); a line in another metric, or a row that names none, is not judged against it but fails,
# since a budget holds its numbers in one unit. A peak row holds a fresh process's full build: `<platform>
# <session> metric <the platform's peak> ceiling_mb <mb> because <why>`, or `... ceiling_ratio <r>
# of <baseline session> because <why>`, the peak at most `r` times the baseline's in the same run.
# What a recording measured stands apart from the ceilings, as `observed <platform> <session>
# metric <m> peak_mb <mb> [ratio <r> of <baseline>] recorded <date>`, read by no judgement. A
# recording rewrites the rows of the sessions it ran and keeps every other; under RECORD=max it
# raises a session's row to the run's growth and memory at the end where they are larger and
# leaves the peak rows as they are, so that several runs record their largest.
#
# A session whose budget the reference machine recorded on its platform alone is declared beside
# its row, `only <platform> <session> because <why>`: on another platform, where it has no row,
# it is not run, where a session that has no row and no such line fails. Its row is judged
# wherever the platform is the row's, on any machine, as every other row is.
#
# The environment: BUDGETS (the file), PLATFORM, TOLERANCE (percent), GROWTH_SLACK (KB per
# edit), FACTOR, FAILED (the sessions that gave no line), PASSED (what passed before the
# sessions ran), RECORD, TEQ_VERSION.
import datetime, os, sys, tempfile

LONG = 200
# The parallel typer's memory aim: a peak at most twice one worker's at eight workers and
# three times at sixteen, one worker's being the plain build's (`@1`); at twelve 2.5, provisional,
# the rows at 12 kept for the cap's next reading while the cap stays 8.
AIMS = {"8": 2.0, "12": 2.5, "16": 3.0}
# A session at the automatic count, named `<session>@auto`, is held to the budget of `<session>@<n>`
# for the count n it logged (its line's `workers`): the budgets of the counts it may choose.
AUTO = "@auto"


def row_of(parts):
    """The fields of a budget's row after its platform and session, and its reason."""
    if "because" in parts:
        at = parts.index("because")
        fields, why = parts[2:at], " ".join(parts[at + 1:])
    else:
        fields, why = parts[2:], ""
    row = {}
    i = 0
    while i < len(fields):
        if fields[i] == "ceiling_ratio":
            row["ceiling_ratio"] = (float(fields[i + 1]), fields[i + 3])
            i += 4
        else:
            row[fields[i]] = fields[i + 1]
            i += 2
    row["because"] = why
    return row


def budgets_of(path, platform):
    budget = {}
    lines = open(path).read().splitlines() if os.path.exists(path) else []
    for line in lines:
        parts = line.split()
        if parts and parts[0] == platform:
            budget[parts[1]] = row_of(parts)
    return lines, budget


def only_of(lines):
    """The sessions whose budget is one platform's alone: the platform and why, by session."""
    only = {}
    for line in lines:
        parts = line.split()
        if len(parts) > 2 and parts[0] == "only":
            only[parts[2]] = (parts[1], " ".join(parts[4:]) if parts[3:4] == ["because"] else "")
    return only


def acceptance(name, edits, budget, only, platform):
    """Whether a session of that many edits runs here: 0 and nothing, 1 and why it cannot be judged,
    or 2 and why it is not run, its budget another platform's alone."""
    why = cannot_judge(name, edits, budget)
    if why and name not in budget and name in only and only[name][0] != platform:
        return 2, f"{name} has its budget on {only[name][0]} alone ({only[name][1]}), and this is {platform}"
    return (1, why) if why else (0, None)


def is_peak(row):
    return "ceiling_mb" in row or "ceiling_ratio" in row


def budget_name(name, s):
    """The row a session's line is judged by: its own, or at the automatic count the row of the count it chose."""
    return name[: -len(AUTO)] + "@" + s["workers"] if name.endswith(AUTO) and "workers" in s else name


def cannot_judge(name, edits, budget):
    """Why a session of that many edits cannot be judged, or nothing; a session at the automatic
    count, whose count is known once it ran, by every row of a count it may choose."""
    if name.endswith(AUTO):
        rows = [n for n in budget if n.startswith(name[: -len(AUTO)] + "@") and n[len(name) - len(AUTO) + 1:].isdigit()]
        if not rows:
            return f"{name} has no budget of a count it may choose; record the rows at counts with WATCH_MEMORY_RECORD=1"
        return next((why for why in (cannot_judge(n, edits, budget) for n in rows) if why), None)
    if name not in budget:
        return f"{name} has no budget; record one with WATCH_MEMORY_RECORD=1"
    if is_peak(budget[name]):
        return None if edits == 0 else f"{name} is a peak row, of a first build and no edit"
    recorded = int(budget[name]["edits"])
    if edits != recorded and edits < LONG:
        return f"{name} would run {edits} edits, which is neither the {recorded} of its budget nor {LONG} and more, whose memory at the end is held against the hundredth edit's"
    return None


def judge_peak(name, s, row, sessions):
    """The failures of a peak session's line."""
    peak = float(s["peak_mb"])
    if "ceiling_mb" in row:
        limit = float(row["ceiling_mb"])
        return [f"{name} peaks at {peak} MB against a ceiling of {limit} ({row['because']})"] if peak > limit else []
    ratio, base = row["ceiling_ratio"]
    if base not in sessions:
        return [f"{name} is held to {ratio} times {base}, which did not run"]
    base_peak = float(sessions[base]["peak_mb"])
    if peak > ratio * base_peak:
        return [f"{name} peaks at {peak} MB, {peak / base_peak:.2f} times {base}'s {base_peak}, against {ratio} ({row['because']})"]
    return []


def unjudged(name, s, budget):
    """Why a session's line cannot be held to a row at all, or nothing: no row of its count, or a row
    of another metric; a run of it again would not change that."""
    if name.endswith(AUTO) and "workers" not in s:
        return f"{name} logged no count of its workers"
    held = budget_name(name, s)
    why = cannot_judge(held, int(s["edits"]), budget)
    if why:
        return why
    measured, held_in = s.get("metric"), budget[held].get("metric")
    if measured is None or held_in is None or measured != held_in:
        return f"{name} measured {measured or 'in no metric named'}, its row holds {held_in or 'no metric'}"
    return None


def judge(name, s, budget, tolerance, slack, factor, sessions=None):
    """The failures of a session's line."""
    why = unjudged(name, s, budget)
    if why:
        return [why]
    edits = int(s["edits"])
    row = budget[budget_name(name, s)]
    if is_peak(row):
        return judge_peak(name, s, row, sessions or {})
    recorded, limit_growth, limit_end = int(row["edits"]), float(row["growth_kb_per_edit"]), float(row["memory_end_mb"])
    growth, end = float(s["growth_kb_per_edit"]), float(s["memory_end_mb"])
    failures = []
    if growth > limit_growth + slack and growth > limit_growth * (1 + tolerance):
        failures.append(f"{name} grows {growth} KB per edit against a budget of {limit_growth}")
    if edits == recorded:
        if end > limit_end * (1 + tolerance):
            failures.append(f"{name} ends at {end} MB against a budget of {limit_end}")
    else:
        hundredth = float(s["memory_100_mb"])
        if end > hundredth * factor:
            failures.append(f"{name} ends at {end} MB after {edits} edits, {end / hundredth:.2f} times the {hundredth} MB after the hundredth (at most {factor})")
    return failures


def baseline_of(name):
    """The plain build's peak session a peak session's aim is a ratio of."""
    session, _, workers = name.rpartition("@")
    return f"{session}@1" if name.startswith("peak-") and workers in AIMS else None


def recorded_rows(platform, sessions, tolerance, version, raise_only=None):
    """The rows and observations a recording writes for the sessions of a run; with the budget
    of `raise_only`, a session's row no lower than the one there and no peak row."""
    rows, observed = [], []
    sessions = {name: v for name, v in sessions.items() if not name.endswith(AUTO)}
    today = datetime.date.today()
    for name, s in sessions.items():
        if s.get("workload") != "peak":
            growth, end = float(s["growth_kb_per_edit"]), float(s["memory_end_mb"])
            old = (raise_only or {}).get(name)
            if old:
                # Raised only by a run of the row's own edits; any other leaves the row as it is.
                if is_peak(old) or old.get("edits") != s["edits"]:
                    continue
                # A row of another metric holds other units: the run's numbers replace it.
                if old.get("metric") == s["metric"]:
                    growth, end = max(growth, float(old["growth_kb_per_edit"])), max(end, float(old["memory_end_mb"]))
            rows.append(f"{platform} {name} edits {s['edits']} growth_kb_per_edit {growth} memory_end_mb {end} metric {s['metric']}")
            continue
        if raise_only is not None:
            continue
        peak = float(s["peak_mb"])
        base = baseline_of(name)
        aim = AIMS.get(name.rpartition("@")[2])
        ratio_note = ""
        if base and base in sessions:
            measured = peak / float(sessions[base]["peak_mb"])
            ratio_note = f" ratio {measured:.2f} of {base}"
            if measured <= aim:
                rows.append(f"{platform} {name} metric {s['metric']} ceiling_ratio {aim} of {base} because the parallel typer's memory aim, at most {aim:g} times one worker's peak; {measured:.2f} measured")
            else:
                limit = round(peak * (1 + tolerance), 1)
                rows.append(f"{platform} {name} metric {s['metric']} ceiling_mb {limit} because the aim of {aim:g} times one worker's peak does not hold: {measured:.2f} measured, {peak} MB, and the tolerance's {tolerance * 100:g}% over it")
        else:
            limit = round(peak * (1 + tolerance), 1)
            rows.append(f"{platform} {name} metric {s['metric']} ceiling_mb {limit} because {peak} MB measured and the tolerance's {tolerance * 100:g}% over it")
        observed.append(f"observed {platform} {name} metric {s['metric']} peak_mb {peak}{ratio_note} recorded {today}, {version}")
    return rows, observed


def record(lines, platform, sessions, tolerance, version, raise_only=None):
    """The budget file's lines with the run's sessions recorded and every other row kept: a
    recorded session's row and observation replaced in place, a new one added after the
    platform's last row."""
    rows, observed = recorded_rows(platform, sessions, tolerance, version, raise_only)
    by_name = {r.split()[1]: r for r in rows}
    names = list(by_name)
    seen_by_name = {o.split()[2]: o for o in observed}
    out, last_row = [], None
    for line in lines:
        parts = line.split()
        if len(parts) > 1 and parts[0] == platform and parts[1] in by_name:
            out.append(by_name.pop(parts[1]))
            last_row = len(out)
        elif len(parts) > 2 and parts[0] == "observed" and parts[1] == platform and parts[2] in seen_by_name:
            out.append(seen_by_name.pop(parts[2]))
        else:
            out.append(line)
            if parts and parts[0] == platform:
                last_row = len(out)
    what = "raised to the run's numbers where larger" if raise_only is not None else "recorded"
    note = [f"# {platform}: {', '.join(names)} {what} {datetime.date.today()}, {version}"]
    added = note + list(by_name.values())
    at = len(out) if last_row is None else last_row
    out[at:at] = added
    return out + list(seen_by_name.values())


def self_test():
    budget = {
        "split-body": {"edits": "40", "growth_kb_per_edit": "600.0", "memory_end_mb": "290.0", "metric": "Rss+Swap"},
        "split-body@8": {"edits": "40", "growth_kb_per_edit": "900.0", "memory_end_mb": "700.0", "metric": "Rss+Swap"},
        "hot-full": {"edits": "20", "growth_kb_per_edit": "600.0", "memory_end_mb": "280.0", "metric": "Rss+Swap"},
        "check-day": {"edits": "40", "growth_kb_per_edit": "200.0", "memory_end_mb": "180.0"},
        "lsp-day": {"edits": "40", "growth_kb_per_edit": "7.0", "memory_end_mb": "8.0", "metric": "Anonymous+Swap"},
        "peak-core@1": {"metric": "VmHWM", "ceiling_mb": "300", "because": "measured"},
        "peak-core@8": {"metric": "VmHWM", "ceiling_ratio": (2.0, "peak-core@1"), "because": "the aim"},
    }
    line = lambda edits, growth, end, hundredth=None, metric="Rss+Swap": {"edits": str(edits), "growth_kb_per_edit": str(growth), "memory_end_mb": str(end), "metric": metric, **({"memory_100_mb": str(hundredth)} if hundredth else {})}
    peak = lambda mb, metric="VmHWM": {"workload": "peak", "edits": "0", "metric": metric, "peak_mb": str(mb)}
    cases = [
        ("a session within its budget", "split-body", line(40, 700, 300), {}, False),
        ("a session that ends over its budget", "split-body", line(40, 0, 5000), {}, True),
        ("a session that grows over its budget", "split-body", line(40, 25000, 300), {}, True),
        ("a growth within the slack", "split-body", line(40, 1600, 300), {}, False),
        ("a session without a budget", "check-body", line(40, 0, 100), {}, True),
        ("a half-length session of another count", "hot-full", line(80, 0, 5000), {}, True),
        ("a long session within the factor", "split-body", line(3000, 10, 310, 300), {}, False),
        ("a long session past the factor", "split-body", line(3000, 10, 1896, 303), {}, True),
        ("a long session that grows", "split-body", line(3000, 2500, 310, 300), {}, True),
        ("a peak under its ceiling", "peak-core@1", peak(290), {}, False),
        ("a peak over its ceiling", "peak-core@1", peak(310), {}, True),
        ("a peak of another metric", "peak-core@1", peak(290, "phys_footprint_peak"), {}, True),
        ("a peak within its ratio of the baseline", "peak-core@8", peak(500), {"peak-core@1": peak(260)}, False),
        ("a peak past its ratio of the baseline", "peak-core@8", peak(530), {"peak-core@1": peak(260)}, True),
        ("a ratio without its baseline", "peak-core@8", peak(100), {}, True),
        ("a peak row asked to run edits", "peak-core@1", {**peak(100), "edits": "40"}, {}, True),
        ("a session at the automatic count within the row of its count", "split-body@auto", {**line(40, 700, 650), "workers": "8"}, {}, False),
        ("a session at the automatic count past the row of its count", "split-body@auto", {**line(40, 0, 900), "workers": "8"}, {}, True),
        ("a session at the automatic count of a count without a row", "split-body@auto", {**line(40, 700, 300), "workers": "3"}, {}, True),
        ("a session at the automatic count that logged no count", "split-body@auto", line(40, 700, 300), {}, True),
        ("a session in another metric than its row's", "split-body", line(40, 700, 250, metric="Anonymous+Swap"), {}, True),
        ("a session whose row names no metric", "check-day", line(40, 0, 100), {}, True),
        ("a session that names no metric against a row that names none", "check-day", {**line(40, 0, 100), "metric": None}, {}, True),
        ("the server's own memory within its budget", "lsp-day", line(40, 7.7, 8.5, metric="Anonymous+Swap"), {}, False),
        # The server's file-backed pages counted in again: 15 MB passed the 13.4 MB row of the
        # resident size, and its anonymous memory is held to its own 8.
        ("the server's own memory over its budget", "lsp-day", line(40, 0, 15.0, metric="Anonymous+Swap"), {}, True),
        ("the server's resident size against its row of anonymous memory", "lsp-day", line(40, 0, 8.0), {}, True),
    ]
    wrong = [what for what, name, s, others, fails in cases if bool(judge(name, s, budget, 0.15, 1500.0, 1.5, others)) != fails]
    # A session whose budget is one platform's alone: run where the platform is the row's, not run
    # on another, and a session without a row or such a line refused.
    only = only_of(["only Linux-x86_64 index-std-day because the reference machine's alone"])
    here = {"index-std-day": {"edits": "40", "growth_kb_per_edit": "1.0", "memory_end_mb": "270.0", "metric": "Rss+Swap"}}
    accepted = [
        ("a session of one platform on its platform", acceptance("index-std-day", 40, here, only, "Linux-x86_64")[0] == 0),
        ("a session of one platform on another", acceptance("index-std-day", 40, {}, only, "Darwin-arm64")[0] == 2),
        ("a session of one platform on its platform without its row", acceptance("index-std-day", 40, {}, only, "Linux-x86_64")[0] == 1),
        ("a session without a row on another platform", acceptance("index-day", 40, {}, only, "Darwin-arm64")[0] == 1),
        ("a session of one platform at another count", acceptance("index-std-day", 80, here, only, "Linux-x86_64")[0] == 1),
    ]
    wrong += [what for what, ok in accepted if not ok]
    # The recording: the run's sessions replaced and added, every other row kept.
    lines = [
        "# Linux-x86_64: recorded before",
        "Linux-x86_64 split-body edits 40 growth_kb_per_edit 1.0 memory_end_mb 100.0 metric Rss+Swap",
        "Linux-x86_64 hot-full edits 20 growth_kb_per_edit 2.0 memory_end_mb 200.0 metric Rss+Swap",
        "Linux-x86_64 lsp-day edits 40 growth_kb_per_edit 7.0 memory_end_mb 13.4 metric Rss+Swap",
        "Darwin-arm64 hot-full edits 20 growth_kb_per_edit 3.0 memory_end_mb 300.0 metric phys_footprint",
    ]
    run = {"hot-full": {"workload": "full", "metric": "Rss+Swap", "edits": "20", "growth_kb_per_edit": "9.0", "memory_end_mb": "210.0"},
           "peak-core@1": peak(250), "peak-core@8": peak(600), "peak-core@16": peak(700)}
    out = record(lines, "Linux-x86_64", run, 0.15, "teq test")
    with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False) as f:
        f.write("\n".join(out) + "\n")
    _, kept = budgets_of(f.name, "Linux-x86_64")
    _, other = budgets_of(f.name, "Darwin-arm64")
    os.unlink(f.name)
    checks = [
        ("the recording keeps a row it did not run", kept.get("split-body", {}).get("memory_end_mb") == "100.0"),
        ("the recording replaces a row it ran", kept.get("hot-full", {}).get("memory_end_mb") == "210.0"),
        ("the recording names the row's metric", kept.get("hot-full", {}).get("metric") == "Rss+Swap"),
        ("the recording keeps another platform's rows", other.get("hot-full", {}).get("memory_end_mb") == "300.0"),
        ("the baseline's ceiling is its peak with the tolerance", kept.get("peak-core@1", {}).get("ceiling_mb") == "287.5"),
        ("a peak past the aim holds its own number", kept.get("peak-core@8", {}).get("ceiling_mb") == "690.0"),
        ("a peak within the aim holds the aim", kept.get("peak-core@16", {}).get("ceiling_ratio") == (3.0, "peak-core@1")),
        ("the observations stand apart", sum(l.startswith("observed Linux-x86_64 peak-core@") for l in out) == 3),
    ]
    raised = record(out, "Linux-x86_64", {"hot-full": {"workload": "full", "metric": "Rss+Swap", "edits": "20", "growth_kb_per_edit": "5.0", "memory_end_mb": "220.0"},
                                          "split-body": {"workload": "body", "metric": "Rss+Swap", "edits": "40", "growth_kb_per_edit": "3.0", "memory_end_mb": "90.0"},
                                          "lsp-day": {"workload": "day", "metric": "Anonymous+Swap", "edits": "40", "growth_kb_per_edit": "6.0", "memory_end_mb": "8.0"},
                                          "peak-core@1": peak(999)}, 0.15, "teq test", kept)
    with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False) as f:
        f.write("\n".join(raised) + "\n")
    _, after = budgets_of(f.name, "Linux-x86_64")
    os.unlink(f.name)
    checks += [
        ("a raising recording keeps the larger growth", after["hot-full"]["growth_kb_per_edit"] == "9.0" and after["hot-full"]["memory_end_mb"] == "220.0"),
        ("a raising recording takes a larger growth", after["split-body"]["growth_kb_per_edit"] == "3.0" and after["split-body"]["memory_end_mb"] == "100.0"),
        ("a raising recording leaves the peak rows", after["peak-core@1"].get("ceiling_mb") == "287.5"),
        ("a raising recording of another metric replaces the row", after["lsp-day"]["memory_end_mb"] == "8.0" and after["lsp-day"]["metric"] == "Anonymous+Swap"),
    ]
    longer = record(raised, "Linux-x86_64", {"hot-full": {"workload": "full", "metric": "Rss+Swap", "edits": "200", "growth_kb_per_edit": "0.1", "memory_end_mb": "1.0"}}, 0.15, "teq test", after)
    with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False) as f:
        f.write("\n".join(longer) + "\n")
    _, last = budgets_of(f.name, "Linux-x86_64")
    os.unlink(f.name)
    checks.append(("a raising recording of other edits leaves the row", last["hot-full"] == after["hot-full"]))
    wrong += [what for what, ok in checks if not ok]
    for what in wrong:
        print(f"FAIL the judge on {what}")
    total = len(cases) + len(accepted) + len(checks)
    print(f"{total - len(wrong)} passed, {len(wrong)} failed")
    return 1 if wrong else 0


def main():
    env = os.environ
    if sys.argv[1] == "self-test":
        return self_test()
    if sys.argv[1] == "baseline":
        base = baseline_of(sys.argv[2])
        if base:
            print(base)
        return 0
    platform = env["PLATFORM"]
    lines, budget = budgets_of(env["BUDGETS"], platform)
    if sys.argv[1] == "accepts":
        code, why = (0, None) if env.get("RECORD") else acceptance(sys.argv[2], int(sys.argv[3]), budget, only_of(lines), platform)
        if why:
            print(why)
        return code
    sessions = {}
    for line in open(sys.argv[2]):
        parts = line.split()
        sessions[parts[0]] = dict(zip(parts[1::2], parts[2::2]))
    tolerance, slack, factor = float(env["TOLERANCE"]) / 100, float(env["GROWTH_SLACK"]), float(env["FACTOR"])
    if sys.argv[1] == "failing":
        for name, s in sessions.items():
            failures = [] if unjudged(name, s, budget) else judge(name, s, budget, tolerance, slack, factor, sessions)
            if failures:
                print(f"{name}\t{failures[0]}")
        return 0
    failed = int(env["FAILED"])
    for name, s in sessions.items():
        joined = f", {s['workers']} workers joined in {s['joined']} full builds" if "workers" in s else ""
        if s.get("workload") == "peak":
            print(f"{name}: a fresh process's full build, peak {s['peak_mb']} MB ({s['metric']}), {s['memory_first_mb']} MB after it, {s['first_build_ms']} ms{joined}")
            continue
        print(f"{name}: {s['incremental']} incremental and {s['full']} full builds, memory ({s.get('metric')}) {s['memory_first_mb']} MB after the first build, "
              f"{s['memory_end_mb']} MB at the end (peak {s['peak_mb']}), growth {s['growth_kb_per_edit']} KB per edit, build {s['build_ms_median']} ms (median), {s['build_ms_max']} ms at most{joined}")
    if env.get("RECORD"):
        if failed:
            print(f"watch-memory: nothing recorded, {failed} sessions failed")
            return 1
        out = record(lines, platform, sessions, tolerance, env["TEQ_VERSION"], budget if env["RECORD"] == "max" else None)
        open(env["BUDGETS"], "w").write("\n".join(out) + "\n")
        print(f"watch-memory: recorded {len(sessions)} sessions for {platform}")
        return 0
    for name, s in sessions.items():
        failures = judge(name, s, budget, tolerance, slack, factor, sessions)
        for failure in failures:
            print(f"FAIL {failure}")
        failed += 1 if failures else 0
    print(f"{len(sessions) + int(env['FAILED']) - failed + int(env.get('PASSED', 0))} passed, {failed} failed ({len(sessions)} sessions against their budgets for {platform})")
    return 1 if failed else 0


sys.exit(main())
