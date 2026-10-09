#!/usr/bin/env python3
"""A check session with the navigation index driven through a fixed script, every answer printed without
its timings, so that two sessions' outputs compare byte for byte (tests/workers.sh, tests/app.sh):

    python3 tests/support/session.py <teq> <file> <text> -- <teq compiler watch arguments...>

The first build; then the file handed in (`text`) with a comment line appended and built, and taken off
again and built, two retypes; then a build that takes the full path (`TEQ_COMPACT_EVERY=3`, the
session's memory); after each, the file's symbols, the references to the first <text> of the file and
the workspace's symbols named <text>. The environment reaches the session, which `TEQ_COMPACT_EVERY`
joins."""
import json, os, subprocess, sys

teq, path, text = sys.argv[1], os.path.abspath(sys.argv[2]), sys.argv[3]
args = sys.argv[sys.argv.index("--") + 1:]
original = open(path, encoding="utf-8").read()
at = original.encode("utf-8").index(text.encode("utf-8"))
env = dict(os.environ, TEQ_COMPACT_EVERY="3")
p = subprocess.Popen([teq, "compiler", "watch", "--check", "--index", *args], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)


def answer(what):
    line = p.stdout.readline()
    if not line:
        print(f"{what}: the session ended")
        sys.exit(1)
    a = json.loads(line)
    a.pop("ms", None)
    print(f"{what}: {json.dumps(a, sort_keys=True)}")


def send(command):
    p.stdin.write(command)
    p.stdin.flush()


def queries(after):
    for q in (f"symbols {path}", f"references {path} {at} 1", f"workspace-symbols {text}"):
        send(q + "\n")
        answer(f"{after}, {q.split()[0]}")


answer("first build")
queries("first build")
for i, body in enumerate((original + "\n// appended\n", original)):
    data = body.encode("utf-8")
    send(f"text {path} {len(data)}\n")
    p.stdin.buffer.write(data)
    send("build\n")
    answer(f"retype {i + 1}")
    queries(f"retype {i + 1}")
send("build\n")
answer("the full path again")
queries("the full path again")
send("quit\n")
p.wait(timeout=60)
