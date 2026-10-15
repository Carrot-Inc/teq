#!/usr/bin/env python3
# The observable contract of a forked test run (docs/TOOLING.md, "Tests."), from either side's transcript, for
# tests/task.sh's comparisons of teq's test runner with sbt 2.0.8's forked worker over the probe framework
# (tests/support/probe_framework):
#   runner_contract.py sbt|teq <token> exact|relaxed <expected> < transcript
# sbt's transcript is ForkTestMain's notifications (JSON-RPC lines) among the suites' stdout, teq's the runner's
# protocol (src/task/runner/Runner.java) among it; both become the same lines: a logger's info as written, its
# warnings and errors after `[warn] ` and `[error] `, a line of stdout as written, and an event as
# `event <task's suite> <status> <name> <selector> <ms> <thrown>`, what it threw its first line then ` @ ` and the
# frames of the suites of `probe.pin` (`<class>.<method>`, comma-separated), `-` for nothing, and a fork error (the
# worker's forkError notification, teq's forkError line) as `forkError <thrown>`. The worker's debug lines are left
# out, as are teq's protocol lines but its events and fork errors. `exact` holds the lines to the expected ones in
# order; `relaxed`, for a run of several workers, holds the events, the fork errors and the error lines to them as a
# multiset and the rest in order. The frames of a stack trace the JVM prints (an exception no one caught) are left
# out, each side's naming its own classes, as are a failed task future's hash and the class of what failed it. Exits
# 1 with both transcripts on a difference.
import json
import re
import sys

STATUSES = ["Success", "Error", "Failure", "Skipped", "Ignored", "Canceled", "Pending"]
WRAPPED = "sbt.internal.worker1.ForkTestMain$ForkError: "
# A task's future that failed, as the worker logs it (`Failed to execute task <future>`): its hash and the class of
# what failed it (gson's, shaded into the worker, where teq's runner has none) left out.
FAILED_TASK = re.compile(r"java\.util\.concurrent\.FutureTask@[0-9a-f]+\[Completed exceptionally: [\w.$]+: ")
# A frame of a stack trace a JVM prints (of an exception no one caught), which names each side's own classes.
PRINTED_FRAME = re.compile(r"^\s+(at |\.\.\. \d+ more)")


def thrown(first, frames):
    if first is None:
        return "-"
    own = [f for f in frames if f.startswith("probe.pin.")]
    return first + (" @ " + ",".join(own) if own else "")


def from_sbt(text):
    out = []
    for line in text.splitlines():
        try:
            message = json.loads(line)
        except ValueError:
            if not PRINTED_FRAME.match(line):
                out.append(line)
            continue
        method, params = message.get("method"), message.get("params", {})
        if method == "testLog":
            tag = params["tag"]
            if tag == "Debug":
                continue
            prefix = {"Info": "", "Warn": "[warn] ", "Error": "[error] "}[tag]
            out.extend(prefix + l for l in params["message"].splitlines())
        elif method == "forkError":
            error = params["error"]
            out.append("forkError " + thrown(error["message"], [f["className"] + "." + f["methodName"] for f in error.get("staceTrace", [])]))
        elif method == "testEvents":
            for e in params["events"]:
                s = e["selector"]
                selector = {"TestSelector": lambda: "test " + s.get("_testName", ""), "SuiteSelector": lambda: "suite"}.get(s["type"], lambda: s["type"])()
                exception = e.get("throwable", {}).get("exception")
                first = frames = None
                if exception:
                    # The event's throwable is the worker's ForkError of the ForkError it made of what the task threw.
                    first = exception["message"]
                    first = first[len(WRAPPED):] if first.startswith(WRAPPED) else first
                    frames = [f["className"] + "." + f["methodName"] for f in exception.get("staceTrace", [])]
                out.append("event %s %s %s %s %d %s" % (params["group"], e["status"], e["fullyQualifiedName"], selector, e["duration"], thrown(first, frames or [])))
    return out


def from_teq(text, token):
    out = []
    pending = []
    for line in text.splitlines():
        before, at, rest = line.partition(token)
        if before and not PRINTED_FRAME.match(before):
            out.append(before)
        if not at:
            continue
        fields = rest.split("\t")[1:]
        kind = fields[0] if fields else ""
        if kind == "throwable":
            pending = [[fields[1], []]]
        elif kind == "cause":
            pending.append([fields[1], []])
        elif kind == "frame" and pending:
            pending[-1][1].append(fields[2].split("(")[0])
        elif kind == "event":
            suite, status, name, selector, a, b, ms = fields[1:8]
            shown = {"test": "test " + a, "suite": "suite", "nestedSuite": "nestedSuite " + a, "nestedTest": "nestedTest %s %s" % (a, b), "wildcard": "wildcard " + a}[selector]
            first, frames = (pending[0][0], pending[0][1]) if pending else (None, [])
            out.append("event %s %s %s %s %s %s" % (suite, STATUSES[int(status)], name, shown, ms, thrown(first, frames)))
            pending = []
        elif kind == "forkError":
            first, frames = (pending[0][0], pending[0][1]) if pending else (None, [])
            out.append("forkError " + thrown(first, frames))
            pending = []
        elif kind == "crashed":
            first, frames = (pending[0][0], pending[0][1]) if pending else (None, [])
            out.append("crashed %s %s" % (fields[1], thrown(first, frames)))
            pending = []
    return out


def same(expected, got, mode):
    if mode == "exact":
        return expected == got
    loose = lambda l: l.startswith("event ") or l.startswith("forkError ") or l.startswith("[error] ")
    return [l for l in expected if not loose(l)] == [l for l in got if not loose(l)] and sorted(filter(loose, expected)) == sorted(filter(loose, got))


def main():
    side, token, mode, expected = sys.argv[1:5]
    text = sys.stdin.read()
    got = [FAILED_TASK.sub("java.util.concurrent.FutureTask[Completed exceptionally: ", l) for l in (from_sbt(text) if side == "sbt" else from_teq(text, token))]
    want = open(expected, encoding="utf-8").read().splitlines()
    if same(want, got, mode):
        return 0
    print("expected:\n  " + "\n  ".join(want) + "\n%s's:\n  " % side + "\n  ".join(got))
    return 1


if __name__ == "__main__":
    sys.exit(main())
