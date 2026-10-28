"""Dotty's own tests of the checks that read the warning policy, run by teq at one worker and under attached workers
(tests/warnings/workers.sh): tests/warn/tostring-interpolated.scala, tests/warn/i23693.scala (-Wtostring-interpolated),
tests/warn/nowarn.scala and tests/warn/i23651.scala (-deprecation -Wunused:nowarn), read from the scala3 checkout at
its tag 3.8.4 (`git show 3.8.4:<path>`, SCALA3), their options those tests/scala3-typing/warn-oracle.jsonl records
scalac's run with, mapped to teq's flags by tests/warnings/policy.py; and tests/neg/nowarn-typer-error.scala, whose
options are its header's. Each test is checked at `--threads 1` and under `TEQ_IDLE_WORKERS=0 --threads 2`: the two
arms' diagnostics, in their order, and exits are the same byte for byte; and each arm's diagnostics (severity, file,
line, column, first line) against scalac's committed ones, the difference, a conformance question of the lint's or the
reporter's, the same in both arms. Usage: workers_dotty.py <teq> <records dir>; prints one line per test and a last
line with the counts; exit 1 on a difference between the arms.
"""
import json
import os
import subprocess
import sys
from collections import Counter
from pathlib import Path

TESTS = ["warn/tostring-interpolated.scala", "warn/i23693.scala", "warn/nowarn.scala", "warn/i23651.scala", "neg/nowarn-typer-error.scala"]


def scala3():
    if os.environ.get("SCALA3"):
        return Path(os.environ["SCALA3"])
    common = subprocess.run(["git", "rev-parse", "--path-format=absolute", "--git-common-dir"], capture_output=True, text=True).stdout.strip()
    return Path(common).parent.parent / "teq-ref" / "scala3"


def oracle():
    found = {}
    for line in Path("tests/scala3-typing/warn-oracle.jsonl").read_text().splitlines():
        if line.startswith("{"):
            entry = json.loads(line)
            found[entry["test"]] = entry
    return found


def header_options(text):
    for line in text.splitlines()[:10]:
        if line.startswith("//> using options"):
            return line[len("//> using options"):].split()
    return []


def run(teq, src, flags, attached, record):
    env = dict(os.environ)
    threads = ["--threads", "1"]
    if attached:
        env["TEQ_IDLE_WORKERS"] = "0"
        threads = ["--threads", "2"]
    env["TEQ_FORK_ITEMS"] = "1"
    diags = record.with_suffix(".json")
    if diags.exists():
        diags.unlink()
    r = subprocess.run([teq, "compiler", "check", str(src), *flags, *threads, "--diagnostics", str(diags)], env=env, capture_output=True, text=True, timeout=120)
    record.with_suffix(".out").write_text(r.stdout + r.stderr)
    lines = diags.read_text().splitlines() if diags.exists() else []
    return r.returncode, lines, r.stderr


def records(lines):
    out = Counter()
    for line in lines:
        if not line.startswith("{"):
            continue
        d = json.loads(line)
        first = d.get("message", "").split("\n")[0]
        out[(d.get("severity"), Path(d.get("file", "")).name, d.get("line"), d.get("col"), first)] += 1
    return out


def scalac_records(entry):
    out = Counter()
    for sev, file, line, col, msg in entry["diagnostics"]:
        out[(sev, file, line, col, (msg[0] if msg else ""))] += 1
    return out


def main():
    teq, out = sys.argv[1], Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    checkout = scala3()
    found = oracle()
    same = 0
    fail = False
    for test in TESTS:
        text = subprocess.run(["git", "-C", str(checkout), "show", f"3.8.4:tests/{test}"], capture_output=True, text=True)
        if text.returncode != 0:
            print(f"skip {test}: not in {checkout} at 3.8.4")
            continue
        src = out / Path(test).name
        src.write_text(text.stdout)
        entry = found.get(test)
        options = entry["options"] if entry else header_options(text.stdout)
        flags = subprocess.run(["python3", "tests/warnings/policy.py", "flags"], input=" ".join(options), capture_output=True, text=True).stdout.split()
        name = test.replace("/", "_")
        one = run(teq, src, flags, False, out / f"{name}.one")
        attached = run(teq, src, flags, True, out / f"{name}.attached")
        if (one[0], one[1]) != (attached[0], attached[1]):
            print(f"FAIL {test} ({' '.join(flags)}): attached workers' diagnostics or exit are not one worker's")
            fail = True
            continue
        if "worker 1:" not in attached[2] or " 0 items" in attached[2].split("worker 1:")[1].split(",")[0]:
            print(f"FAIL {test}: no attached worker typed an item")
            fail = True
            continue
        same += 1
        if entry:
            mine, theirs = records(one[1]), scalac_records(entry)
            missing, extra = theirs - mine, mine - theirs
            print(f"{test}: one worker's at attached workers; against scalac 3.8.4: {sum(missing.values())} missing, {sum(extra.values())} extra (exit {one[0]}, scalac's {entry['exit']})")
        else:
            print(f"{test}: one worker's at attached workers (exit {one[0]})")
    print(f"dotty's tests: {same} of {len(TESTS)} as one worker's under attached workers")
    sys.exit(1 if fail else 0)


if __name__ == "__main__":
    main()
