#!/usr/bin/env python3
"""Lists the members of the class files under a directory as `javap -s -p` describes them, one
`Class name descriptor` line each, sorted: a constructor by its class's name, a field by its
name, the static initializer as `<clinit>`. The names of `excluded` (one per line, `#` comments)
are left out, and so are the classes of a package object (`$package`), which scalac writes only
for top-level definitions.

usage: javap_members.py <directory> [excluded]
"""
import os
import re
import subprocess
import sys


def members(paths):
    """The members of each class file, javap's blocks taken in the order of its arguments."""
    out = subprocess.run(['javap', '-s', '-p'] + paths, capture_output=True, text=True, timeout=120).stdout
    blocks, found, pending = [], [], None
    for line in out.splitlines():
        s = line.strip()
        if line == '}':
            blocks.append(found)
            found, pending = [], None
        elif s.startswith('descriptor:'):
            if pending is not None:
                found.append((pending, s.split(':', 1)[1].strip()))
            pending = None
        elif s == 'static {};':
            pending = '<clinit>'
        elif s.endswith(';'):
            m = re.search(r'([\w$]+)\s*\(', s) or re.search(r'([\w$]+);$', s)
            pending = m.group(1) if m else None
    if len(blocks) != len(paths):
        sys.exit(f'javap described {len(blocks)} of {len(paths)} class files')
    return blocks


def main():
    root = sys.argv[1]
    excluded = set()
    if len(sys.argv) > 2:
        for line in open(sys.argv[2]):
            name = line.split('#', 1)[0].strip()
            if name:
                excluded.add(name)
    paths = sorted(os.path.join(d, f) for d, _, files in os.walk(root) for f in files if f.endswith('.class'))
    paths = [p for p in paths if '$package' not in p]
    lines = set()
    for path, found in zip(paths, members(paths)):
        cls = os.path.relpath(path, root)[:-len('.class')]
        simple = cls.rsplit('/', 1)[-1]
        for name, desc in found:
            if name in excluded:
                continue
            name = '<init>' if name == simple else name
            lines.add(f'{cls} {name} {desc}')
    for line in sorted(lines):
        print(line)


main()
