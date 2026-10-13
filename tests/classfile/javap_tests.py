#!/usr/bin/env python3
"""The type tests of the methods of the class files under a directory, as `javap -c -p` shows them:
one line per method, `<class> <method>: <instructions>`, of its `instanceof` (with the class), `ifnull`,
`ifnonnull` and `checkcast` instructions in order; methods without one are left out. Usage:
javap_tests.py <dir> [<class name prefix>]"""
import os, re, subprocess, sys

def main():
    root = sys.argv[1]
    prefix = sys.argv[2] if len(sys.argv) > 2 else ''
    files = []
    for d, _, names in os.walk(root):
        files += [os.path.join(d, n) for n in names if n.endswith('.class')]
    out = []
    for f in sorted(files):
        name = os.path.relpath(f, root)[:-len('.class')]
        if not name.startswith(prefix):
            continue
        text = subprocess.run(['javap', '-c', '-p', f], capture_output=True, text=True, check=True).stdout
        method, tests = None, []
        for line in text.splitlines():
            m = re.match(r'^  (?:[\w$<>\[\],. ]+ )?([\w$<>]+)\(.*\);$', line)
            if m:
                if method and tests:
                    out.append(f'{name} {method}: {" ".join(tests)}')
                method, tests = m.group(1), []
                continue
            i = re.match(r'^\s+\d+: (instanceof|checkcast)\s+#\d+\s+// class (\S+)', line)
            if i:
                tests.append(f'{i.group(1)} {i.group(2)}')
            elif re.match(r'^\s+\d+: (ifnull|ifnonnull)\b', line):
                tests.append(line.split(':', 1)[1].split()[0])
        if method and tests:
            out.append(f'{name} {method}: {" ".join(tests)}')
    print('\n'.join(out))

main()
