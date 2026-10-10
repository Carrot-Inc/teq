# The comparison of tests/warnings/unused-kinds.sh: a compiler's unused warnings for one file of
# dotty's tests/warn, every kind (scalac's E198 `Unused Symbol` messages and the warning of an unused
# `@nowarn`), one `<line>:<col>:<width> <message>` per warning in source order, the columns from 1
# where the carets start; and the options of a file's `//> using options` lines for each compiler.
#   kinds.py scalac <file> < scalac's output    (-color:never)
#   kinds.py teq <file> < teq's output
#   kinds.py options scalac < file              scalac's options, one per line: the -Wunused kinds,
#                                                -Wconf, -language, -source and -Y but -Ystop-after
#   kinds.py options teq < file                 teq's flags for them, on one line
#   kinds.py strip < file > file'                the file with those lines blanked
import re, shlex, sys

MESSAGES = re.compile(r"^(unused (import|local definition|explicit parameter|implicit parameter|private member|pattern variable)|local variable was mutated but not read|private variable was mutated but not read|unset (local|private) variable, consider using an immutable val instead|@nowarn annotation does not suppress any warnings( but matches a diagnostic)?)")

def options(text):
    kept = []
    lines = text.split("\n")
    for i, line in enumerate(lines):
        if line.startswith("//> using options"):
            for o in shlex.split(line[len("//> using options"):]):
                if o.startswith("-Wunused") or o.startswith("-Wconf") or o.startswith("-language") or o.startswith("-source") or (o.startswith("-Y") and not o.startswith("-Ystop-after")):
                    kept.append(o)
            lines[i] = "\r" if line.endswith("\r") else ""
    return kept, "\n".join(lines)

def teq_flags(kept):
    flags = []
    for o in kept:
        if o.startswith("-Wunused:"):
            flags += ["--wunused", o[len("-Wunused:"):]]
        elif o == "-Wunused":
            flags += ["--wunused", "all"]
        elif o.startswith("-Wconf:"):
            flags += ["--wconf", o[len("-Wconf:"):]]
        elif o.startswith("-language:"):
            flags += ["--language", o[len("-language:"):]]
    return flags

def parse(mode, path, lines):
    found = []
    name = path.split("/")[-1]
    if mode == "scalac":
        head = re.compile(r"^-- (?:\[E\d+\] )?[A-Za-z ]*?Warning: (.*):(\d+):(\d+) -*$")
        i = 0
        while i < len(lines):
            m = head.match(lines[i])
            if not m or not m.group(1).endswith(name):
                i += 1
                continue
            col = width = msg = None
            j = i + 1
            while j < len(lines) and re.match(r"^(\d+ |\s+)\|", lines[j]):
                c = re.match(r"^\s*\|(\s*)(\^+)\s*$", lines[j])
                if c and col is None:
                    col, width = len(c.group(1)) + 1, len(c.group(2))
                elif col is not None and msg is None:
                    msg = lines[j].split("|", 1)[1].strip()
                j += 1
            if msg and MESSAGES.match(msg):
                found.append((int(m.group(2)), col, width, msg))
            i = j
    else:
        head = re.compile(r"^(.*):(\d+):(\d+): warning: (.*)$")
        for i, line in enumerate(lines):
            m = head.match(line)
            if not m or not m.group(1).endswith(name) or not MESSAGES.match(m.group(4)):
                continue
            width = 1
            for j in range(i + 1, min(i + 8, len(lines))):
                c = re.match(r"^\s*(\^+)\s*$", lines[j])
                if c:
                    width = len(c.group(1))
                    break
            found.append((int(m.group(2)), int(m.group(3)), width, m.group(4).strip()))
    return sorted(set(found))

mode = sys.argv[1]
if mode == "options":
    kept, _ = options(sys.stdin.buffer.read().decode("utf-8"))
    if sys.argv[2] == "scalac":
        print("\n".join(kept))
    else:
        print(" ".join(shlex.quote(f) for f in teq_flags(kept)))
elif mode == "strip":
    _, text = options(sys.stdin.buffer.read().decode("utf-8"))
    sys.stdout.buffer.write(text.encode("utf-8"))
else:
    for l, c, w, msg in parse(mode, sys.argv[2], sys.stdin.read().split("\n")):
        print(f"{l}:{c}:{w} {msg}")
