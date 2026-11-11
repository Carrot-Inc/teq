#!/usr/bin/env python3
"""How tests/workers.sh's session_flat reads a session's growth:

  session_flat.py session <teq> [NAME=value ...] [argument ...]
  session_flat.py series <readings file> [<bytes> | <bytes>@<build>]

`session` runs a watch session over tests/split/cycle (the arguments its own, a NAME=value its environment's) through
109 full builds and prints the growth of the allocator's blocks in use (`stats`): the lowest of the readings after
the builds 90 to 109 less the lowest after the builds 11 to 30, a build's reading the higher of two `stats` asked
one after the other.

Two things move a reading that are no growth, both measured on master's binary (sessions of 120 builds, alone and
under the JVM suite's load):

- The blocks in use sit on one of two levels some 4.4 to 5 KB apart, more than the 2 KB the check allows: an object
  a build leaves or does not, the same for every `stats` after that build. A build is on the high level one time
  in ten on a quiet machine and three in ten on a loaded one, where the level stays for runs of builds (nineteen
  in a row were seen in 160 loaded sessions, no twenty). The lowest of twenty builds is the low level's on both
  sides; the lowest of three, which the check read before, was the high level's on one side alone in 7% of the
  loaded sessions: a growth of one level that no build made.
- About one `stats` in thirty-one answers some 750 bytes lower, that one alone: the next one after the same build
  answers as the others (of 8,400 builds asked twice, 560 had two answers apart, all by 700 to 800 bytes, none
  across the levels). The higher of two is the build's. A single reading a build let such an answer into one side
  alone, and the lowest of twenty then read 544 to 848 bytes low, which let a growth of 2,700 bytes pass that the
  reading of three refused.

So read, the check's own session with no growth read -128 to 192 bytes in 170 runs under that load and -336 to 368
in 210 alone (the reading of three: -464 to 1,152 where it did not misread by a level), and a growth is read to
within that: in 70 sessions recorded under the load, asked after every build, a step of 2,200 bytes between the
two sides was refused in every one. A growth of g bytes a build reads as 60 g or more:
the lowest early build is the thirtieth or an earlier one, the lowest late one the ninetieth or a later one, as
far apart as the two readings were before. What the reading can lose is one level, where every early build is on
the high one: 10 of 144 loaded sessions for three builds, none seen for twenty.

`series` reads the same growth off a file of readings, a build a line, its two answers on it, the first build's
first, each raised by the bytes named for every build up to its own, or, as <bytes>@<build>, by those bytes from
that build on: the reading's decision with no compiler in it, which tests/workers.sh shows on two sessions
measured under load (tests/workers/session_flat_levels.txt, which the lowest of three read as 4,800 bytes of
growth, and tests/workers/session_flat_dip.txt, which the lowest of twenty single answers read as -720) and on
each of them growing.
"""
import json
import os
import subprocess
import sys

EARLY = range(11, 31)
LATE = range(90, 110)


def growth(readings):
    """The growth read off `readings`, the answers after each build by its number."""
    return min(max(readings[i]) for i in LATE) - min(max(readings[i]) for i in EARLY)


def session(teq, arguments):
    env = {k: v for k, v in os.environ.items() if not k.startswith("TEQ_")}
    env["TEQ_COMPACT_EVERY"] = "1"
    args = []
    for a in arguments:
        if "=" in a:
            k, v = a.split("=", 1)
            env[k] = v
        else:
            args.append(a)
    p = subprocess.Popen([teq, "compiler", "watch", "--check", "tests/split/cycle", *args], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)

    def send(c):
        p.stdin.write(c + "\n")
        p.stdin.flush()
        return json.loads(p.stdout.readline())

    json.loads(p.stdout.readline())
    readings = {}
    for i in range(1, LATE[-1] + 1):
        send("build")
        if i in EARLY or i in LATE:
            readings[i] = [send("stats")["stats"]["allocator"]["live"] for _ in range(2)]
    p.stdin.write("quit\n")
    p.stdin.flush()
    p.wait(timeout=30)
    return readings


def series(path, grown):
    """The file's answers by build, raised by `grown`: `<bytes>` a build, or `<bytes>@<build>` from that build on."""
    if "@" in grown:
        step, start = (int(x) for x in grown.split("@"))
        raised = lambda i: step if i >= start else 0
    else:
        raised = lambda i: int(grown) * i
    with open(path) as f:
        return {i: [int(x) + raised(i) for x in line.split()] for i, line in enumerate(f, 1)}


if __name__ == "__main__":
    if len(sys.argv) >= 3 and sys.argv[1] == "session":
        print(growth(session(sys.argv[2], sys.argv[3:])))
    elif len(sys.argv) in (3, 4) and sys.argv[1] == "series":
        print(growth(series(sys.argv[2], sys.argv[3] if len(sys.argv) == 4 else "0")))
    else:
        sys.exit(__doc__)
