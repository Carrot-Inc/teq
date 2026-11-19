#!/usr/bin/env python3
"""tests/java-lang-names/run.py <teq> <scala-library jar> <out> [jobs]: each cell of cells.tsv (gen.py) built and run
by <teq> on four legs: JavaScript (`compiler build -o`, node), JavaScript over scala-library (`--std=scala-library`),
the interpreter (`interp`), the JVM (`--target jvm`, `java TeqMain` against the jar). A cell passes when every leg
prints scalac's output, and a cell scalac refuses (`REFUSED`) when every leg refuses it; else its first leg that does
not, with its first line. -> <out>/results.txt, the lines
tests/support/pending.py reads (`pass <cell>`, `fail <cell>\\t<why>`), a cell named `<class>__<position>`."""
import concurrent.futures, os, re, shutil, subprocess, sys
TEQ, SL, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
JOBS = int(sys.argv[4]) if len(sys.argv) > 4 else os.cpu_count() or 4
HERE = os.path.dirname(os.path.abspath(__file__))
def run(cmd, timeout=120):
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, errors='replace')
        return r.returncode, r.stdout, r.stderr
    except subprocess.TimeoutExpired:
        return 'timeout', '', ''
def first(text, w):
    lines = [l for l in text.replace(w + '/', '').splitlines() if l.strip()]
    return next((l for l in lines if 'error' in l), lines[0] if lines else '').strip()[:140]
def one(cell):
    name, pos, src, want = cell
    if want.startswith('REFUSED'):
        return refused(cell)
    key = f'{name}__{pos}'
    w = os.path.join(OUT, 'w', key)
    shutil.rmtree(w, ignore_errors=True)
    os.makedirs(w)
    f = os.path.join(w, 'Main.scala')
    open(f, 'w').write(src)
    why = None
    for leg, build in (('JavaScript', ['-o', os.path.join(w, 'a.js')]), ('JavaScript over scala-library', ['--std=scala-library', '-o', os.path.join(w, 'b.js')])):
        e, o, er = run([TEQ, 'compiler', 'build', f] + build)
        if e == 0:
            e, o, er = run(['node', build[-1]], 60)
        if e != 0 or o.strip() != want:
            why = why or f'{leg}: ' + (first(o + er, w) if e != 0 else f'printed {o.strip()[:60]!r}')
    e, o, er = run([TEQ, 'interp', f])
    if e != 0 or o.strip() != want:
        why = why or 'interpreter: ' + (first(o + er, w) if e != 0 else f'printed {o.strip()[:60]!r}')
    jv = os.path.join(w, 'jvm')
    e, o, er = run([TEQ, 'compiler', 'build', f, '--target', 'jvm', '-o', jv, '--classpath', SL])
    if e == 0:
        e, o, er = run(['java', '-XX:-UsePerfData', '-cp', f'{jv}:{SL}', 'TeqMain'], 60)
    if e != 0 or o.strip() != want:
        why = why or 'JVM: ' + (first(o + er, w) if e != 0 else f'printed {o.strip()[:60]!r}')
    shutil.rmtree(w, ignore_errors=True)
    return f'pass {key}' if why is None else f"fail {key}\t{why.replace(chr(9), ' ')} (scalac: {want.replace(chr(10), ' | ')[:40]})"
def refused(cell):
    """A cell scalac refuses: every leg is to refuse it (the build, the interpreter) with an error line of its own; an
    exit that is a timeout or a crash without one is no refusal."""
    name, pos, src, want = cell
    key = f'{name}__{pos}'
    w = os.path.join(OUT, 'w', key)
    shutil.rmtree(w, ignore_errors=True)
    os.makedirs(w)
    f = os.path.join(w, 'Main.scala')
    open(f, 'w').write(src)
    legs = (('JavaScript', [TEQ, 'compiler', 'check', f]), ('JavaScript over scala-library', [TEQ, 'compiler', 'check', f, '--std=scala-library']),
            ('interpreter', [TEQ, 'interp', f]), ('JVM', [TEQ, 'compiler', 'check', f, '--target', 'jvm', '--classpath', SL]))
    why = None
    for leg, cmd in legs:
        e, o, er = run(cmd)
        if e == 0:
            why = why or f'{leg}: accepted, printed {o.strip()[:40]!r}' if leg == 'interpreter' else why or f'{leg}: accepted'
        elif e == 'timeout' or not any(': error: ' in l for l in (o + er).splitlines()):
            why = why or f'{leg}: no error line (exit {e}: {first(o + er, w)[:60]})'
    shutil.rmtree(w, ignore_errors=True)
    return f'pass {key}' if why is None else f"fail {key}\t{why} ({want[:80]})"
cells = []
for l in open(os.path.join(HERE, 'cells.tsv')):
    if l.startswith('#'):
        continue
    name, pos, src, want = l.rstrip('\n').split('\t')
    unescape = lambda t: re.sub(r'\\(.)', lambda m: '\n' if m.group(1) == 'n' else m.group(1), t)
    cells.append((name, pos, unescape(src), unescape(want)))
os.makedirs(OUT, exist_ok=True)
with concurrent.futures.ThreadPoolExecutor(JOBS) as ex:
    lines = list(ex.map(one, cells))
open(os.path.join(OUT, 'results.txt'), 'w').write('\n'.join(lines) + '\n')
print(f'{sum(l.startswith("pass") for l in lines)} pass, {sum(l.startswith("fail") for l in lines)} fail of {len(lines)} cells')
