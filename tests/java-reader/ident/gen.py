#!/usr/bin/env python3
"""tests/java-reader/ident/gen.py [--check]: runs Ident.java on the JDK on the PATH and writes src/java/ident.rs, the
ranges of the code points a Java source's identifier may start with and go on with, which the Java scanner reads
(src/java/scanner.rs). With --check it writes nothing: the file must equal what this JDK gives, over all 1,114,112
code points, and the count of code points apart is printed (0, or the check fails)."""
import os, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(HERE)))
OUT = os.path.join(ROOT, 'src', 'java', 'ident.rs')
with tempfile.TemporaryDirectory() as d:
    subprocess.run(['javac', '-d', d, os.path.join(HERE, 'Ident.java')], check=True)
    lines = subprocess.run(['java', '-cp', d, 'Ident'], check=True, capture_output=True, text=True).stdout.splitlines()
version = lines[0].lstrip('# ').strip()
ranges = {'start': [], 'part': []}
for l in lines[1:]:
    kind, lo, hi = l.split()
    ranges[kind].append((int(lo), int(hi)))
def table(name, rs):
    body = ''.join(f'    ({lo:#x}, {hi:#x}),\n' for lo, hi in rs)
    return f'pub const {name}: &[(u32, u32)] = &[\n{body}];\n'
text = (f'//! The code points a Java source\'s identifier may start with (`START`) and go on with (`PART`), as scalac 3.8.4\'s\n'
        f'//! Java scanner reads them, in ranges: written by tests/java-reader/ident/gen.py from {version}\n'
        f'//! (Ident.java beside it), which the suite checks again. Do not edit by hand.\n\n'
        + table('START', ranges['start']) + '\n' + table('PART', ranges['part']) + '\n'
        + '/// Whether `c` may start an identifier.\npub fn is_start(c: char) -> bool {\n    within(START, c as u32)\n}\n\n'
        + '/// Whether `c` may go on an identifier.\npub fn is_part(c: char) -> bool {\n    within(PART, c as u32)\n}\n\n'
        + 'fn within(ranges: &[(u32, u32)], c: u32) -> bool {\n    ranges.binary_search_by(|&(lo, hi)| if hi < c { std::cmp::Ordering::Less } else if lo > c { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Equal }).is_ok()\n}\n')
if '--check' in sys.argv:
    def points(src):
        import re
        out = {}
        for name in ('START', 'PART'):
            block = src.split(f'pub const {name}')[1].split('];')[0]
            s = set()
            for lo, hi in re.findall(r'\((0x[0-9a-f]+), (0x[0-9a-f]+)\)', block):
                s.update(range(int(lo, 16), int(hi, 16) + 1))
            out[name] = s
        return out
    have, want = points(open(OUT).read()), points(text)
    apart = sum(len(have[n] ^ want[n]) for n in ('START', 'PART'))
    print(f'java-sources ident: {apart} code points apart from {version} over all 1114112 (start and part)')
    sys.exit(0 if apart == 0 else 1)
open(OUT, 'w').write(text)
print(f'wrote {OUT}: {len(ranges["start"])} start ranges, {len(ranges["part"])} part ranges, {version}')
