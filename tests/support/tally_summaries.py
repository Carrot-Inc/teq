#!/usr/bin/env python3
"""The transcript a stand-in teq prints for the test verbs' checks (tests/lila-test-verb.sh, tests/mill-test-verb.sh),
its summaries counted as teq's tally counts them (task::events::Tally, src/task/daemon.rs `summary`): tally_summaries.py
<transcript> prints the transcript with each `<unit>/test: <n> suites, <p> passed, <f> failed[, ...] in ...` summary's
counts those of the outcome lines of its section (`[success]` passed, `[fail]` failed, then each skip category that is
not zero), its suites and its times kept; every other line as it is."""
import re, sys

counts = {}
summary = re.compile(r"^(\S+/test: [0-9]+ suites?, )[0-9]+ passed, [0-9]+ failed(?:, [0-9]+ (?:ignored|skipped|pending|canceled))*( in .*)$")
outcome = re.compile(r"^\[(success|skipped|ignored|canceled|pending|fail)\] (.*)$")
for line in open(sys.argv[1]):
    line = line.rstrip("\n")
    plain = re.sub(r"\x1b\[[0-9;]*m", "", line)
    m = summary.match(plain)
    if m:
        others = "".join(f", {counts[k]} {k}" for k in ("ignored", "skipped", "pending", "canceled") if counts.get(k))
        print(f"{m.group(1)}{counts.get('success', 0)} passed, {counts.get('fail', 0)} failed{others}{m.group(2)}")
        counts = {}
        continue
    o = outcome.match(plain)
    if o and not o.group(2).startswith(" "):
        counts[o.group(1)] = counts.get(o.group(1), 0) + 1
    print(line)
