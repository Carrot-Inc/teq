#!/usr/bin/env python3
"""The events a stand-in teq writes for tests/mill-test-verb.sh's runs (`teq test --events`, task::events::Event::line):
mill_events.py <work> <module> reads <work>/transcript.txt and prints an event line for each of its outcomes, the
module, the suite (the longest of the scenario's suites the case names), `test <case>`, the status (`[fail]` a
Failure) and nothing thrown, then a line `<module>\t<suite>\tran` for each suite that ran (task::events::Tally::ran_line),
one the transcript heads (`<suite>:`) or one of a case; <work>/events-override.txt, where it is there, in their
place."""
import os, re, sys

work, module = sys.argv[1], sys.argv[2]
override = os.path.join(work, "events-override.txt")
if os.path.exists(override):
    sys.stdout.write(open(override).read())
    sys.exit(0)
cases = ["p.OneSuite.adds", "p.OneSuite.skips", "p.TwoSuite.parses: twice", "p.IgnoredSuite.<unknown>"]
suites = ["p.EmptySuite", "p.IgnoredSuite", "p.OneSuite", "p.TwoSuite"]
headed, ran = [], []
for line in open(os.path.join(work, "transcript.txt")):
    line = re.sub(r"\x1b\[[0-9;]*m", "", line.rstrip("\n"))
    if re.match(r"^[A-Za-z_$][A-Za-z0-9_$.]*:$", line):
        headed.append(line[:-1])
    m = re.match(r"^\[(success|skipped|ignored|canceled|pending|fail)\] (.*)$", line)
    if not m or m.group(2).startswith(" "):
        continue
    kind, rest = m.groups()
    name = max((c for c in cases if rest == c or rest.startswith(c + ": ")), key=len, default=rest.split(":")[0]) if kind == "fail" else rest
    suite = max((s for s in suites if name.startswith(s + ".")), key=len, default=name.rsplit(".", 1)[0])
    print(f"{module}\t{suite}\ttest {name}\t{'Failure' if kind == 'fail' else kind.capitalize()}\t")
    ran.append(suite)
# Each suite that ran (`task::events::Tally::ran_line`), after the cases: one its heading names (`<suite>:`, an empty
# suite's too) or a case of it.
for suite in dict.fromkeys(headed + ran):
    print(f"{module}\t{suite}\tran")
