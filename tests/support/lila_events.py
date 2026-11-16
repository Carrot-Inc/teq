#!/usr/bin/env python3
"""The events a stand-in teq writes for tests/lila-test-verb.sh's runs (`teq test --events`, task::events::Event::line):
lila_events.py <work> reads <work>/transcript.txt and prints an event line for each of its outcomes: the project whose
summary (`<project>/test: ...`) ends its section, the suite (the longest of the scenario's suites the line names),
`test <case>` (a failure's throwable after `: ` left out; a line of the suite alone its `suite` selector), the status
(`[fail]` a Failure) and nothing thrown, then a line `<project>\t<suite>\tran` for each suite of the section
(task::events::Tally::ran_line); an outcome after the last summary none. A case's event is named as lila's framework
names it (munit's junit-interface: `<suite>.<case>` as the event's name and as its selector's test), which the driver
reads as sbt's JUnit report names it (`Tests.asReports`). <work>/events-override.txt, where it
is there, in their place."""
import os, re, sys

work = sys.argv[1]
override = os.path.join(work, "events-override.txt")
if os.path.exists(override):
    sys.stdout.write(open(override).read())
    sys.exit(0)
suites = ["lila.a.OneTest", "lila.a.TwoTest", "lila.b.ThreeTest"]
cases = ["adds", "subtracts: twice", "parses", "renders"]
section = []
for line in open(os.path.join(work, "transcript.txt")):
    line = line.rstrip("\n")
    summary = re.match(r"^([^ ]+)/test: [0-9]+ suites?, ", line)
    if summary:
        for _, named, selector, status in section:
            print(f"{summary.group(1)}\t{named}\t{selector}\t{status}\t")
        # Each suite of the section ran (`task::events::Tally::ran_line`), after the cases.
        for suite in dict.fromkeys(s for s, _, _, _ in section):
            print(f"{summary.group(1)}\t{suite}\tran")
        section = []
        continue
    m = re.match(r"^\[(success|skipped|ignored|canceled|pending|fail)\] (.*)$", line)
    if not m or m.group(2).startswith(" ") or m.group(2).startswith(" at "):
        continue
    kind, rest = m.groups()
    suite = max((s for s in suites if rest == s or rest.startswith(s + " ") or rest.startswith(s + ":")), key=len, default=rest.split(" ")[0])
    test = rest[len(suite):].lstrip(" ")
    if test.startswith(":") or not test:
        selector = "suite"
    else:
        if kind == "fail":
            test = max((c for c in cases if test == c or test.startswith(c + ": ")), key=len, default=test.split(":")[0])
        selector = f"test {test}"
    named = suite if selector == "suite" else f"{suite}.{test}"
    section.append((suite, named, selector if selector == "suite" else f"test {named}", "Failure" if kind == "fail" else kind.capitalize()))
