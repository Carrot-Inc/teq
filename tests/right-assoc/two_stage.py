#!/usr/bin/env python3
"""tests/right-assoc/two_stage.py <teq> <scala-library jar> <scalac class path> <out dir> [--only <prefix>]...: the
two-stage builds of tests/right-assoc's cells whose operator is defined above `run` (or in Defs.scala), both ways,
against scalac 3.8.4's whole build (tests/right-assoc/expected.tsv). Run by hand where scalac is (needs a JDK and
scalac's class path); its table is evidence for a landing's message, not a line of the gate:
  teq-scalac  teq builds the definitions to products; scalac compiles the call against them (the products' `scala/`
              tree, teq's own runtime classes, off its class path) and runs `run`;
  scalac-teq  scalac builds the definitions; teq builds the call against scalac's class files and TASTy, on the JVM,
              and runs it.
Each build bounded (teq 120 s, scalac 300 s), each run 60 s. -> <out dir>/two-stage.tsv: cell, leg, line, verdict.
The judge's self-test runs first."""
import concurrent.futures, os, shutil, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import lines  # noqa: E402
from run import bounded, result_line, judged  # noqa: E402


def split(cell_dir, work):
    names = sorted(f for f in os.listdir(cell_dir) if f.endswith('.scala'))
    for f in names:
        shutil.copy(os.path.join(cell_dir, f), os.path.join(work, f))
    if names == ['Test.scala']:
        text = open(os.path.join(work, 'Test.scala')).read()
        lib = lines.lib(text)
        if not lib:
            return None
        open(os.path.join(work, 'Lib.scala'), 'w').write(lib)
        open(os.path.join(work, 'Client.scala'), 'w').write(text[len(lib):])
        return ['Lib.scala'], ['Client.scala']
    return ['Defs.scala'], [f for f in names if f != 'Defs.scala']


def legs(teq, sl, sc, cell, cell_dir, work):
    os.makedirs(work, exist_ok=True)
    s = split(cell_dir, work)
    if s is None:
        return {}
    libs, clients = s
    res = {}
    scalac = ['java', '-XX:-UsePerfData', '-cp', sc, 'dotty.tools.dotc.Main', '-color:never']
    c, o, e = bounded([teq, 'compiler', 'build'] + libs + ['--target', 'jvm', '--classpath', sl, '--products', 'prod'], work, 120)
    if c == 0:
        shutil.copytree(os.path.join(work, 'prod'), os.path.join(work, 'prod-sc'),
                        ignore=lambda d, n: ['scala'] if os.path.samefile(d, os.path.join(work, 'prod')) and 'scala' in n else [])
        os.makedirs(os.path.join(work, 'tc'))
        c, o, e = bounded(scalac + ['-classpath', os.path.join(work, 'prod-sc') + ':' + sl, '-d', 'tc'] + clients, work, 300)
        if c == 0:
            c, o, e = bounded(['java', '-XX:-UsePerfData', '-cp', ':'.join([os.path.join(work, 'tc'), os.path.join(work, 'prod'), sl]), 'run'], work, 60)
    res['teq-scalac'] = result_line(cell, c, o, e)
    os.makedirs(os.path.join(work, 'sl'))
    c, o, e = bounded(scalac + ['-classpath', sl, '-d', 'sl'] + libs, work, 300)
    if c == 0:
        c, o, e = bounded([teq, 'compiler', 'build'] + clients + ['--target', 'jvm', '--classpath', os.path.join(work, 'sl') + ':' + sl, '-o', 'cl'], work, 120)
        if c == 0:
            c, o, e = bounded(['java', '-XX:-UsePerfData', '-Xss512m', '-cp', ':'.join([os.path.join(work, 'cl'), os.path.join(work, 'sl'), sl]), 'TeqMain'], work, 60)
    res['scalac-teq'] = result_line(cell, c, o, e)
    return res


def main():
    a = sys.argv[1:]
    teq, sl, sc, out = os.path.abspath(a[0]), a[1], a[2], os.path.abspath(a[3])
    only = [a[i + 1] for i, x in enumerate(a) if x == '--only']
    assert judged('c\trun\tb1 n1,b', 'c\trun\tb1 n1,b') and not judged('c\trun\tb1 n1,b', 'c\trun\tb1 b,n1')
    print('two-stage: self-test: 2 of 2 planted results judged as planted')
    shutil.rmtree(out, ignore_errors=True)
    os.makedirs(out)
    subprocess.run([sys.executable, os.path.join(HERE, 'gen.py'), os.path.join(out, 'table')], check=True, capture_output=True)
    cells = [l.split('\t')[0] for l in open(os.path.join(out, 'table', 'cells.tsv')) if l.strip()]
    if only:
        cells = [c for c in cells if any(c.startswith(p) for p in only)]
    expected = {l.split('\t')[0]: l for l in open(os.path.join(HERE, 'expected.tsv')).read().splitlines() if l and not l.startswith('#')}
    rows = []
    with concurrent.futures.ThreadPoolExecutor(int(os.environ.get('JOBS', '8'))) as ex:
        for cell, res in ex.map(lambda c: (c, legs(teq, sl, sc, c, os.path.join(out, 'table', 'cells', c), os.path.join(out, 'work', c))), cells):
            for leg, line in res.items():
                exp = expected.get(cell)
                v = 'no captured line' if exp is None else ('as scalac' if judged(exp, line) else 'DIFFERS')
                rows.append((cell, leg, line.split('\t', 1)[1], v))
    with open(os.path.join(out, 'two-stage.tsv'), 'w') as fh:
        fh.write('# cell\tleg\tline\tverdict\n')
        for r in rows:
            fh.write('\t'.join(x.replace('\t', ' ') for x in r) + '\n')
    from collections import Counter
    for (leg, v), n in sorted(Counter((r[1], r[3]) for r in rows).items()):
        print(n, leg, v)


if __name__ == '__main__':
    main()
