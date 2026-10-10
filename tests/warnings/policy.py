# The comparison of tests/warnings/policy.sh: a compiler's diagnostics normalized to one line
# each, `exit N` first, then sorted `<severity> <line>:<col> <first line of the message>` and
# `<severity> <message's first line>` for one of no place, the columns from 1 where the carets
# start. scalac's `No warnings can be incurred under -Werror` error is left out: teq says so in
# its own words, and the exit carries it.
#   policy.py scalac <exit> <file> < scalac's output    (-color:never)
#   policy.py teq <exit> <file> < teq's output
#   policy.py flags < scalac options                     teq's flags for them, one line
import re, shlex, sys

def teq_flags(options):
    out = []
    for o in options:
        if o == "-deprecation":
            out.append("--deprecation")
        elif o == "-feature":
            out.append("--feature")
        elif o == "-Werror":
            out.append("--werror")
        elif o == "-Wtostring-interpolated":
            out.append("--wtostring-interpolated")
        elif o.startswith("-Wunused:"):
            out += ["--wunused", o[len("-Wunused:"):]]
        elif o.startswith("-Wconf:"):
            out += ["--wconf", o[len("-Wconf:"):]]
        elif o.startswith("-language:"):
            out += ["--language", o[len("-language:"):]]
        else:
            raise SystemExit(f"no teq flag for {o}")
    return out

# teq names its own options where scalac names its.
WORDING = [("--deprecation", "-deprecation"), ("--feature", "-feature"), ("`--wconf`", "`-Wconf`"), ("--wconf", "-Wconf")]

def scalac(lines, path):
    found = []
    name = path.split("/")[-1]
    head = re.compile(r"^-- (?:\[E\d+\] )?[A-Za-z ]*?(Warning|Error|Info): (.*):(\d+):(\d+) -*$")
    bare = re.compile(r"^-- (?:\[E\d+\] )?[A-Za-z ]*?(Warning|Error|Info): -*$")
    i = 0
    while i < len(lines):
        line = lines[i]
        m = head.match(line)
        if m and m.group(2).endswith(name):
            severity = m.group(1).lower()
            col = None
            msg = None
            j = i + 1
            while j < len(lines) and re.match(r"^(\d+ |\s+)\|", lines[j]):
                c = re.match(r"^\s*\|(\s*)(\^+)\s*$", lines[j])
                if c and col is None:
                    col = len(c.group(1)) + 1
                elif col is not None and msg is None and lines[j].lstrip().startswith("|"):
                    msg = lines[j].lstrip()[1:].strip()
                j += 1
            found.append(f"{severity} {m.group(3)}:{col} {msg}")
            i = j
            continue
        if bare.match(line):
            severity = bare.match(line).group(1).lower()
            j = i + 1
            msg = None
            while j < len(lines) and re.match(r"^(\d+ |\s+)\|", lines[j]):
                if msg is None and lines[j].lstrip().startswith("|"):
                    msg = lines[j].lstrip()[1:].strip()
                j += 1
            if msg != "No warnings can be incurred under -Werror":
                found.append(f"{severity} {msg}")
            i = j
            continue
        # A message of no place without a header (the summaries, a configuration's failure).
        if re.match(r"^(there (was|were) \d+ .* warnings?; re-run with|Failed to parse `-Wconf`)", line):
            found.append(f"warning {line.strip()}")
        i += 1
    return found

def teq(lines, path):
    found = []
    name = path.split("/")[-1]
    m_head = re.compile(r"^(.*):(\d+):(\d+): (warning|error|info): (.*)$")
    for i, line in enumerate(lines):
        m = m_head.match(line)
        if m and m.group(1).endswith(name):
            msg = m.group(5).strip()
            for a, b in WORDING:
                msg = msg.replace(a, b)
            found.append(f"{m.group(4)} {m.group(2)}:{m.group(3)} {msg}")
            continue
        g = re.match(r"^(warning|error|info): (.*)$", line)
        if g:
            msg = g.group(2).strip()
            for a, b in WORDING:
                msg = msg.replace(a, b)
            found.append(f"{g.group(1)} {msg}")
    return found

mode = sys.argv[1]
if mode == "flags":
    print(" ".join(shlex.quote(f) for f in teq_flags(sys.stdin.read().split())))
    sys.exit(0)
code, path = sys.argv[2], sys.argv[3]
lines = sys.stdin.read().split("\n")
found = scalac(lines, path) if mode == "scalac" else teq(lines, path)
print(f"exit {code}")
for f in sorted(found):
    print(f)
