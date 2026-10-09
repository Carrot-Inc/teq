# The options of a test file's `//> using options` lines for the unused-import comparison.
#   options.py strip < file > file'   the file with those lines blanked (lines keep their numbers)
#   options.py scalac < file          scalac's options: -Wunused:imports and what changes what
#                                     compiles (-source, -language, -Y but -Ystop-after), one per line
#   options.py teq < file             teq's flags for those, on one line
# The warning options, -Werror, -deprecation and -feature are dropped.
import shlex, sys

mode = sys.argv[1]
text = sys.stdin.buffer.read().decode("utf-8")
lines = text.split("\n")
kept = []
for i, line in enumerate(lines):
    if line.startswith("//> using options"):
        opts = shlex.split(line[len("//> using options"):])
        kept += [o for o in opts if not (o.startswith("-W") or o in ("-deprecation", "-feature") or o.startswith("-Ystop-after"))]
        lines[i] = "\r" if line.endswith("\r") else ""
if mode == "strip":
    sys.stdout.buffer.write("\n".join(lines).encode("utf-8"))
elif mode == "scalac":
    print("\n".join(["-Wunused:imports"] + kept))
else:
    flags = []
    for o in kept:
        if o == "-language:strictEquality":
            flags.append("--strict-equality")
    print(" ".join(flags))
