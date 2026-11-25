#!/usr/bin/env python3
"""tests/right-assoc/run.py <teq> <scala-library jar> <out dir> [--only <prefix>]... [--update|--add]: the legs of
every cell of tests/right-assoc/gen.py against scalac 3.8.4's lines (tests/right-assoc/expected.tsv and
descriptors.tsv, captured by tests/right-assoc/capture.sh), each bounded (a teq build or check 120 s, a run 60 s):
  check    `teq compiler check`: accepted where scalac runs, or refused with scalac's first error (its line, its
           column and its message's first line);
  js       built to JavaScript and run by node; interp, teq's interpreter; jvm, built for the JVM and run by java;
           each the line of scalac's run, or its refusal;
  split    the definitions above `run` built to products (--products), then the call against them, run by java;
  workers  the interpreter at 8 workers (TEQ_TYPER_THRESHOLD=0), the line of scalac's run, or its refusal;
  desc     the descriptors of the split build's products for the cell's definitions, scalac's.
A leg is `<cell>.<leg>` in tests/pending/right-assoc.txt (tests/support/pending.py): the suite fails where a leg that
is not scalac's is unlisted, or a listed one now is. A cell without a captured line is skipped and named. The judge's
self-test of planted results runs first. Writes under <out dir> alone; the last line gives the counts."""
import concurrent.futures, os, shutil, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
import lines  # noqa: E402

LEGS = ('check', 'js', 'interp', 'jvm', 'split', 'workers', 'desc')


def bounded(cmd, cwd, timeout, env=None):
    try:
        r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout, env=env)
        return r.returncode, r.stdout, r.stderr
    except subprocess.TimeoutExpired:
        return 'timeout', '', ''


def judged(expected, got):
    """Whether teq's line `got` (a `run`/`refused` line, or `crash ...`) is scalac's `expected`. A refusal is scalac's
    where its line and its message's first line are; the column is compared too. A run is scalac's where scalac's run
    exited 0 and teq's did, with the same lines: a scalac line of a run that crashed or was cut at its bound is no
    expectation, and no leg passes against it."""
    return expected.split('\t')[1] in ('run', 'refused') and got == expected


def leg_results(cell, exp, desc, res):
    """The cell's legs judged: `pass <cell>.<leg>` or `fail <cell>.<leg>\t<teq's line>`. A cell without scalac's line
    fails every leg."""
    if exp is None:
        return [f'fail {cell}.{leg}\tno captured line' for leg in LEGS]
    out = []
    kind = exp.split('\t')[1]
    for leg in LEGS:
        if leg not in res:
            continue
        if leg == 'desc':
            ok = res[leg] == (desc or f'{cell}\t')
        elif leg == 'check':
            ok = res[leg] == exp if kind == 'refused' else res[leg] == f'{cell}\trun\t'
        else:
            ok = judged(exp, res[leg])
        out.append(f'pass {cell}.{leg}' if ok else f'fail {cell}.{leg}\t' + res[leg].split('\t', 1)[1][:200])
    return out


def result_line(cell, code, out, err):
    if code == 'timeout':
        return f'{cell}\ttimeout\t'
    if code == 0:
        return lines.run_line(cell, out)
    if code == 1 and ('error' in err or 'error' in out):
        return lines.refused_line(cell, err + out)
    # A crash's first line, its paths written from the tree's root, so that a list made on one machine holds on another.
    first = (err.strip().splitlines() or [''])[0].replace(REPO + os.sep, '')
    return f'{cell}\tcrash {code}\t' + first[:160]


def legs_of(teq, sl, cell, src, work):
    """`src`, the cell's directory: its Test.scala, or several files, `Defs.scala` the definitions of a cell of
    several."""
    os.makedirs(work, exist_ok=True)
    names = sorted(f for f in os.listdir(src) if f.endswith('.scala'))
    for f in names:
        shutil.copy(os.path.join(src, f), os.path.join(work, f))
    res = {}
    c, o, e = bounded([teq, 'compiler', 'check'] + names, work, 120)
    res['check'] = f'{cell}\trun\t' if c == 0 else result_line(cell, c, o, e)
    c, o, e = bounded([teq, 'compiler', 'build'] + names + ['-o', 'a.js'], work, 120)
    if c == 0:
        c, o, e = bounded(['node', 'a.js'], work, 60)
    res['js'] = result_line(cell, c, o, e)
    c, o, e = bounded([teq, 'interp'] + names, work, 120)
    res['interp'] = result_line(cell, c, o, e)
    c, o, e = bounded([teq, 'compiler', 'build'] + names + ['--target', 'jvm', '--classpath', sl, '-o', 'jv'], work, 120)
    if c == 0:
        c, o, e = bounded(['java', '-XX:-UsePerfData', '-Xss512m', '-cp', os.path.join(work, 'jv') + ':' + sl, 'TeqMain'], work, 60)
    res['jvm'] = result_line(cell, c, o, e)
    env = dict(os.environ, TEQ_TYPER_THRESHOLD='0')
    c, o, e = bounded([teq, 'interp'] + names + ['--threads', '8'], work, 120, env)
    res['workers'] = result_line(cell, c, o, e)
    if names == ['Test.scala']:
        text = open(os.path.join(work, 'Test.scala')).read()
        libtext = lines.lib(text)
        libs, clients = (['Lib.scala'], ['Client.scala']) if libtext else ([], [])
        if libtext:
            open(os.path.join(work, 'Lib.scala'), 'w').write(libtext)
            open(os.path.join(work, 'Client.scala'), 'w').write(text[len(libtext):])
    else:
        libs, clients = ['Defs.scala'], [f for f in names if f != 'Defs.scala']
    if libs:
        c, o, e = bounded([teq, 'compiler', 'build'] + libs + ['--target', 'jvm', '--classpath', sl, '--products', 'prod'], work, 120)
        if c == 0:
            classes = sorted(os.path.join(dp, f) for dp, _, fs in os.walk(os.path.join(work, 'prod')) for f in fs
                             if f.endswith('.class') and os.sep + 'scala' + os.sep not in dp + os.sep)
            jc, jo, je = bounded(['javap', '-s', '-p'] + classes, work, 120) if classes else (0, '', '')
            res['desc'] = lines.descriptors(cell, jo) or f'{cell}\t'
            c, o, e = bounded([teq, 'compiler', 'build'] + clients + ['--target', 'jvm', '--classpath', os.path.join(work, 'prod') + ':' + sl, '-o', 'cl'], work, 120)
            if c == 0:
                c, o, e = bounded(['java', '-XX:-UsePerfData', '-Xss512m', '-cp', ':'.join([os.path.join(work, 'cl'), os.path.join(work, 'prod'), sl]), 'TeqMain'], work, 60)
            res['split'] = result_line(cell, c, o, e)
        else:
            res['split'] = result_line(cell, c, o, e)
            res['desc'] = f'{cell}\tlibrary refused'
    return res


def self_test():
    planted = [
        (('c\trun\tb1 n1,b', 'c\trun\tb1 n1,b'), True),
        (('c\trun\tb1 n1,b', 'c\trun\tb1 b,n1'), False),
        (('c\trefused\t4:19: Not found: l', 'c\trefused\t4:19: not found: l'), False),
        (('c\trun\tb1 n1,b', 'c\tcrash 134\t'), False),
        # a trailing failure: scalac's lines printed, then a crash
        (('c\trun\tb1 n1,b', result_line('c', 1, 'b1 n1,b\n', 'Exception in thread "main" java.lang.Error\n')), False),
        # a run cut at its bound after scalac's lines
        (('c\trun\tb1 n1,b', result_line('c', 'timeout', 'b1 n1,b\n', '')), False),
        # scalac's own run crashed or was cut: no leg passes against it, a like line of teq's included
        (('c\tcrash 1\t', 'c\tcrash 1\t'), False),
        (('c\ttimeout\t', 'c\ttimeout\t'), False),
    ]
    for (exp, got), want in planted:
        assert judged(exp, got) == want, (exp, got)
    crash = result_line('c', 1, '', os.path.join(REPO, 'out', 'w', 'a.js') + ':5\n')
    assert crash == 'c\tcrash 1\tout/w/a.js:5', crash
    # a cell without scalac's line fails every leg, whatever teq's legs gave
    missing = leg_results('c', None, None, {leg: 'c\trun\tb1' for leg in LEGS})
    assert len(missing) == len(LEGS) and all(x.startswith('fail ') for x in missing), missing
    return len(planted) + 2


def main():
    a = sys.argv[1:]
    teq, sl, out = os.path.abspath(a[0]), a[1], os.path.abspath(a[2])
    mode = [x for x in a[3:] if x in ('--update', '--add')]
    only = [a[i + 1] for i, x in enumerate(a) if x == '--only']
    n = self_test()
    print(f'right-assoc: self-test: {n} of {n} planted results judged as planted')
    shutil.rmtree(out, ignore_errors=True)
    os.makedirs(out)
    r = subprocess.run([sys.executable, os.path.join(HERE, 'gen.py'), os.path.join(out, 'table')], capture_output=True, text=True)
    if r.returncode != 0:
        print(r.stdout + r.stderr)
        return 1
    cells = [l.split('\t')[0] for l in open(os.path.join(out, 'table', 'cells.tsv')) if l.strip()]
    if only:
        cells = [c for c in cells if any(c.startswith(p) for p in only)]
    def table(path):
        d = {}
        for l in open(path).read().splitlines():
            if l and not l.startswith('#'):
                d[l.split('\t')[0]] = l
        return d
    expected = table(os.path.join(HERE, 'expected.tsv'))
    descs = table(os.path.join(HERE, 'descriptors.tsv'))
    results, missing = [], []
    def one(cell):
        return cell, legs_of(teq, sl, cell, os.path.join(out, 'table', 'cells', cell), os.path.join(out, 'work', cell))
    with concurrent.futures.ThreadPoolExecutor(int(os.environ.get('JOBS', '8'))) as ex:
        for cell, res in ex.map(one, cells):
            exp = expected.get(cell)
            if exp is None:
                missing.append(cell)
            results.extend(leg_results(cell, exp, descs.get(cell), res))
    rp = os.path.join(out, 'results.txt')
    open(rp, 'w').write('\n'.join(results) + '\n')
    for c in missing:
        print(f'right-assoc: no captured line for {c}')
    r = subprocess.run([sys.executable, os.path.join(REPO, 'tests', 'support', 'pending.py'), 'right-assoc', rp] + mode)
    passed = sum(1 for x in results if x.startswith('pass '))
    failed = sum(1 for x in results if x.startswith('fail '))
    print(f'right-assoc: {len(cells)} cells, {passed} legs pass, {failed} fail' + (' as tests/pending/right-assoc.txt lists' if r.returncode == 0 else ', NOT as listed'))
    return r.returncode


if __name__ == '__main__':
    sys.exit(main())
