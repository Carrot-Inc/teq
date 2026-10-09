# Reads a compiler's rendered diagnostics on stdin and prints its warnings about the cases of
# matches for the file named as the second argument, one `line:col kind` per warning in source
# order, as often as it is given (columns from 1, the start of the span the caret line marks):
# `exhaustive` (scalac's E029, teq's `match may not be exhaustive`), `unreachable` (E030 and
# E121, `unreachable case`) and `not-partial` (E211, `match expression in result of block will
# not be used to synthesize partial function`), from scalac's `-- [E<n>] ... Warning:
# <path>:<line>:<col>` blocks or teq's `<path>:<line>:<col>: warning: <message>` lines.
import re, sys

mode, path = sys.argv[1], sys.argv[2]
name = path.split("/")[-1]
lines = sys.stdin.read().split("\n")
found = []
if mode == "scalac":
    kinds = {"E029": "exhaustive", "E030": "unreachable", "E121": "unreachable", "E211": "not-partial"}
    head = re.compile(r"^-- \[(E\d+)\] [A-Za-z ]*Warning: (.*):(\d+):(\d+) ")
    for i, line in enumerate(lines):
        m = head.match(line)
        if not m or m.group(1) not in kinds or not m.group(2).endswith(name):
            continue
        for j in range(i + 1, min(i + 8, len(lines))):
            c = re.match(r"^\s*\|(\s*)\^+\s*$", lines[j])
            if c:
                found.append((int(m.group(3)), len(c.group(1)) + 1, kinds[m.group(1)]))
                break
else:
    kinds = [("match may not be exhaustive", "exhaustive"), ("unreachable case", "unreachable"),
             ("match expression in result of block will not be used", "not-partial")]
    head = re.compile(r"^(.*):(\d+):(\d+): warning: (.*)$")
    for line in lines:
        m = head.match(line)
        if not m or not m.group(1).endswith(name):
            continue
        for prefix, kind in kinds:
            if m.group(4).startswith(prefix):
                found.append((int(m.group(2)), int(m.group(3)), kind))
for l, c, k in sorted(found):
    print(f"{l}:{c} {k}")
