# Reads a compiler's rendered diagnostics on stdin and prints its `unused import` warnings for the
# file named as the second argument, one `line:col:width` per warning in source order (columns
# from 1, the start of the span): scalac's
# `-- [E198] Unused Symbol Warning: <path>:<line>:<col>` blocks and teq's
# `<path>:<line>:<col>: warning: unused import` lines, the width the caret line's.
import os, re, sys

mode, path = sys.argv[1], sys.argv[2]
# A probe of several files (a directory): every file of a directory of that name, each line
# prefixed with the file's name.
directory = os.path.basename(path.rstrip("/")) if not path.endswith(".scala") else None
def ours(file):
    if directory is None:
        return file.endswith(path.split("/")[-1])
    return os.path.basename(os.path.dirname(file)) == directory
def key(file):
    return os.path.basename(file) + ":" if directory is not None else ""
lines = sys.stdin.read().split("\n")
found = []
if mode == "scalac":
    # The header's column is the span's point (a rename's is at `as`): the caret line under the
    # source line gives the start, counted from the gutter's `|`, and the width.
    head = re.compile(r"^-- \[E198\] Unused Symbol Warning: (.*):(\d+):(\d+) ")
    for i, line in enumerate(lines):
        m = head.match(line)
        if not m or not ours(m.group(1)):
            continue
        for j in range(i + 1, min(i + 8, len(lines))):
            c = re.match(r"^\s*\|(\s*)(\^+)\s*$", lines[j])
            if c:
                found.append((key(m.group(1)), int(m.group(2)), len(c.group(1)) + 1, len(c.group(2))))
                break
else:
    head = re.compile(r"^(.*):(\d+):(\d+): warning: unused import$")
    for i, line in enumerate(lines):
        m = head.match(line)
        if not m or not ours(m.group(1)):
            continue
        width = 1
        if i + 2 < len(lines):
            c = re.match(r"^\s*(\^+)\s*$", lines[i + 2])
            if c:
                width = len(c.group(1))
        found.append((key(m.group(1)), int(m.group(2)), int(m.group(3)), width))
for f, l, c, w in sorted(set(found)):
    print(f"{f}{l}:{c}:{w}")
