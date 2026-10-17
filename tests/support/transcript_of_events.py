#!/usr/bin/env python3
"""The transcript a stand-in teq prints for tests/reference-test-verb.sh's runs, as `teq test --verbose` prints one
(task::events::report, src/task/daemon.rs `summary`): transcript_of_events.py <events> renders each event of the file
(the project, the suite, the selector, the status, the thrown, tab-separated) as its outcome line, `[<status>] <test>`
or `[fail] <test>: <thrown>`, each project's lines together in the order it first came, ended by its summary,
`<project>/test: <n> suites, <p> passed, <f> failed[, <n> ignored, ...] in 1 ms (built in 1 ms)`, counted as teq's
tally counts them."""
import sys

def named(name, selector):
    test = selector[5:] if selector.startswith("test ") else ""
    if not test or test == name:
        return name
    return test if test.startswith(name) else f"{name} {test}"

sections = {}
for line in open(sys.argv[1]):
    line = line.rstrip("\n")
    if not line:
        continue
    if line.split("\t")[2:] == ["ran"]:
        continue
    project, suite, selector, status, thrown = (line.split("\t") + [""] * 5)[:5]
    sections.setdefault(project, []).append((suite, selector, status, thrown))
for project, events in sections.items():
    for suite, selector, status, thrown in events:
        what = named(suite, selector)
        if status in ("Failure", "Error"):
            print(f"[fail] {what}: {thrown}" if thrown else f"[fail] {what}")
        else:
            print(f"[{status.lower()}] {what}")
    passed = sum(1 for e in events if e[2] == "Success")
    failed = sum(1 for e in events if e[2] in ("Failure", "Error"))
    others = "".join(f", {n} {k.lower()}" for k in ("Ignored", "Skipped", "Pending", "Canceled") for n in [sum(1 for e in events if e[2] == k)] if n)
    suites = len({e[0] for e in events})
    print(f"{project}/test: {suites} suite{'' if suites == 1 else 's'}, {passed} passed, {failed} failed{others} in 1 ms (built in 1 ms)")
