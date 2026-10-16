#!/usr/bin/env python3
"""The member references of the methods of the class files under a directory, as `javap -c -p` shows
them: one line per method, `<class> <method>: <instruction> <owner>.<member>:<descriptor> ...`, of its
invoke and field instructions, in order, whose member is one of the names given; methods without one are
left out, and the lines are sorted, so that the order a compiler writes methods in does not count. A
lambda's method is `$anonfun` whatever its number and prefix (scalac's `main$$anonfun$1`, teq's
`$anonfun$1`); the methods `excluded` names (`<class> <method>` lines, `#` comments) are left out. Usage:
javap_owners.py <dir> <class name prefix> <name,name,...> [excluded]"""
import os, re, subprocess, sys

def main():
    root, prefix = sys.argv[1], sys.argv[2]
    names = set(sys.argv[3].split(','))
    excluded = set()
    if len(sys.argv) > 4:
        for line in open(sys.argv[4]):
            line = line.split('#', 1)[0].strip()
            if line:
                excluded.add(tuple(line.split()))
    files = []
    for d, _, ns in os.walk(root):
        files += [os.path.join(d, n) for n in ns if n.endswith('.class')]
    out = []
    for f in sorted(files):
        cname = os.path.relpath(f, root)[:-len('.class')]
        if not cname.startswith(prefix):
            continue
        text = subprocess.run(['javap', '-c', '-p', f], capture_output=True, text=True, check=True).stdout
        method, refs = None, []
        for line in text.splitlines():
            m = re.match(r'^  (?:[\w$<>\[\],.? ]+ )?([\w$<>]+)\(.*\)( throws [\w.$, ]+)?;$', line)
            if m:
                if method and refs and (cname, method) not in excluded:
                    out.append(f'{cname} {method}: {" ".join(refs)}')
                method, refs = re.sub(r'^(?:[\w$]*\$\$)?\$?anonfun\$\d+$', '$anonfun', m.group(1)), []
                continue
            i = re.match(r'^\s+\d+: (invoke\w+|getfield|putfield|getstatic|putstatic)\s+#\d+(?:,\s*\d+)?\s+// (?:Method|InterfaceMethod|Field) (\S+)', line)
            if i:
                target = i.group(2)
                # A member of the class itself is printed without its owner.
                if '.' not in target.split(':')[0]:
                    target = f'{cname}.{target}'
                if target.split(':')[0].rsplit('.', 1)[-1] in names:
                    refs.append(f'{i.group(1)} {target}')
        if method and refs and (cname, method) not in excluded:
            out.append(f'{cname} {method}: {" ".join(refs)}')
    print('\n'.join(sorted(out)))

main()
