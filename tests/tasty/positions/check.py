"""Compares `teq tasty --positions` of every file the oracle's output covers with what scalac's
unpickler gave its trees (compare.py), one file at a time. A file the oracle could not read fails
unless it is one of the known ones given after `--unread`, each with its reason in tests/tasty.sh.

usage: check.py <teq> <oracle output> [--requested <file of paths>] [--skip <substring>...]
[--unread <substring>...]; with --requested, a requested file the oracle gave no answer for fails."""
import subprocess
import sys
from compare import differences, oracle, teq

args = sys.argv[3:]
skip, unread, requested, into = [], [], [], None
for a in args:
    if a in ('--skip', '--unread', '--requested'):
        into = {'--skip': skip, '--unread': unread, '--requested': requested}[a]
    else:
        into.append(a)
wanted = [l.strip() for f in requested for l in open(f) if l.strip()]
blocks, current = {}, None
for line in open(sys.argv[2]):
    if line.startswith('# '):
        current = line[2:].strip()
        blocks[current] = []
    elif current is not None:
        blocks[current].append(line)
passed = failed = 0
answered = {name.partition(': ')[0] for name in blocks}
for path in wanted:
    if path not in answered and not any(s in path for s in skip):
        failed += 1
        print(f'FAIL positions {path}: scalac\'s unpickler gave no answer for it')
for name, lines in blocks.items():
    path, _, problem = name.partition(': ')
    if any(s in path for s in skip):
        continue
    if problem:
        if any(s in path for s in unread):
            continue
        failed += 1
        print(f'FAIL positions {path}: scalac\'s unpickler: {problem}')
        continue
    run = subprocess.run([sys.argv[1], 'tasty', '--positions', path], capture_output=True, text=True)
    diff = differences(teq(run.stdout.splitlines()), oracle(lines)) if run.returncode == 0 else [run.stderr.strip()]
    if diff:
        failed += 1
        print(f'FAIL positions {path}')
        print('\n'.join(diff[:10]))
    else:
        passed += 1
print(f'positions: {passed} passed, {failed} failed')
sys.exit(1 if failed or not passed else 0)
