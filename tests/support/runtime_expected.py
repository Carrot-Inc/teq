#!/usr/bin/env python3
"""Runtime-only expectation for runners that capture compiler warnings separately."""
import re
import sys
from pathlib import Path

_HEADER = re.compile(rb"[^\r\n]+?:[0-9]+:[0-9]+: (warning|error): [^\r\n]+\r?\n\Z")
_CARET = re.compile(rb" *\^+ *(?:\r?\n)?\Z")


def runtime_expected(data):
    lines = data.splitlines(keepends=True)
    first = 0
    while first + 2 < len(lines):
        header = _HEADER.fullmatch(lines[first])
        if not header or header[1] != b"warning" or not _CARET.fullmatch(lines[first + 2]):
            break
        first += 3
    return b"".join(lines[first:])


if __name__ == "__main__":
    sys.stdout.buffer.write(runtime_expected(Path(sys.argv[1]).read_bytes()))
