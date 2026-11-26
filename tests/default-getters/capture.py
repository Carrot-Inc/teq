#!/usr/bin/env python3
"""tests/default-getters/capture.py [--jobs N]: scalac 3.8.4's lines for the cells of tests/default-getters/gen.py,
the suite's expectations. Run by hand where scalac is (coursier and a JDK), when a cell or the generator changes:
each cell's program (Test.scala, or a split cell's Lib.scala and Main.scala together) and, where it has one, its member
twin (Twin.scala) built by scalac (bounded at 300 s) and run by java (60 s), the main class `run` (an `@main def run`)
or `Main`; its line by tests/right-assoc/lines.py (the repository's one place a result becomes a line): `run` and the
output's lines joined by ` | `, `crash <exit>` or `timeout`, or `refused` with the first error's line, column and
message. Written to tests/default-getters/expected.tsv as `<cell>\\t<kind>\\t<line>`, the twin's as `<cell>~twin`;
for a split cell also scalac's class files of its library alone, their default getters' descriptors (desc.py) as
`<cell>~desc`. With `--only <prefix>`, only those cells are captured again and the file's other lines kept.
SCALA_LIBRARY and SCALAC_CLASSPATH stand for coursier's answers. Writes under out/default-getters-capture."""
import concurrent.futures, os, re, shutil, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(REPO, 'tests', 'right-assoc'))
import lines  # noqa: E402


def fetch(env, args):
    v = os.environ.get(env)
    if v:
        return v
    r = subprocess.run(['cs', 'fetch', '--classpath'] + args, capture_output=True, text=True)
    return r.stdout.strip().splitlines()[-1] if r.returncode == 0 and r.stdout.strip() else ''


def bounded(cmd, t, cwd=None):
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=t, cwd=cwd)
        return r.returncode, r.stdout, r.stderr
    except subprocess.TimeoutExpired:
        return 'timeout', '', ''


def main_class(files):
    """`run` for an `@main def run`, else `Main`, in its file's package."""
    texts = [open(f).read() for f in files]
    if any('@main def run()' in x for x in texts):
        return 'run'
    for x in texts:
        if re.search(r'^object Main\b', x, re.M):
            m = re.search(r'^package ([\w.]+)', x, re.M)
            return m.group(1) + '.Main' if m else 'Main'
    return 'Main'


def capture(sc, sl, work, key, files):
    d = os.path.join(work, key.replace('~', '_'))
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(d)
    # From the cell's own directory, its files by their names: scalac's error header names the file short, as teq's
    # legs do (a long path drops the header's dashes, which the line's parser reads).
    cwd = os.path.dirname(files[0])
    code, out, err = bounded(['java', '-XX:-UsePerfData', '-cp', sc, 'dotty.tools.dotc.Main', '-color:never', '-classpath', sl, '-d', d]
                             + [os.path.basename(f) for f in files], 300, cwd)
    if code != 0:
        return lines.refused_line(key, out + err)
    code, out, err = bounded(['java', '-XX:-UsePerfData', '-cp', d + ':' + sl, main_class(files)], 60)
    return lines.run_line(key, out, code)


def capture_desc(sc, sl, work, key, lib):
    """scalac's class files of a split cell's library alone, their default getters' descriptors (desc.py)."""
    d = os.path.join(work, key.replace('~', '_'))
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(d)
    code, out, err = bounded(['java', '-XX:-UsePerfData', '-cp', sc, 'dotty.tools.dotc.Main', '-color:never', '-classpath', sl, '-d', d, lib], 300)
    if code != 0:
        return f'{key}\trefused\t'
    classes = sorted(os.path.join(dp, f) for dp, _, fs in os.walk(d) for f in fs if f.endswith('.class'))
    _, jout, _ = bounded(['javap', '-s', '-p'] + classes, 120) if classes else (0, '', '')
    r = subprocess.run([sys.executable, os.path.join(HERE, 'desc.py')], input=jout, capture_output=True, text=True)
    return f'{key}\tdesc\t{r.stdout.strip()}'


def main():
    jobs = int(sys.argv[sys.argv.index('--jobs') + 1]) if '--jobs' in sys.argv else 8
    only = [sys.argv[i + 1] for i, x in enumerate(sys.argv) if x == '--only']
    sl = fetch('SCALA_LIBRARY', ['--intransitive', 'org.scala-lang:scala-library:3.8.4'])
    sc = fetch('SCALAC_CLASSPATH', ['org.scala-lang:scala3-compiler_3:3.8.4'])
    if not sl or not sc:
        sys.exit('default-getters capture: no scalac 3.8.4 from coursier')
    out = os.path.join(REPO, 'out', 'default-getters-capture')
    shutil.rmtree(out, ignore_errors=True)
    os.makedirs(out)
    table = os.path.join(out, 'table')
    subprocess.run([sys.executable, os.path.join(HERE, 'gen.py'), table], check=True)
    work = os.path.join(out, 'work')
    jobs_list = []
    descs = []
    for l in open(os.path.join(table, 'cells.tsv')):
        cell = l.split('\t')[0]
        if only and not any(cell.startswith(p) for p in only):
            continue
        cd = os.path.join(table, 'cells', cell)
        # The descriptors of getters something outside can name: the two-module witness's top-level classes (a local
        # class's names are each compiler's own design: nothing outside its block names them).
        if 'split' in l.rstrip('\n').split('\t')[2] and cell.startswith('SPLIT-witness-'):
            descs.append((cell + '~desc', os.path.join(cd, 'Lib.scala')))
        fs = sorted(os.listdir(cd))
        if 'Test.scala' in fs:
            jobs_list.append((cell, [os.path.join(cd, 'Test.scala')]))
        else:
            jobs_list.append((cell, [os.path.join(cd, f) for f in fs if f.endswith('.scala') and f != 'Twin.scala']))
        if 'Twin.scala' in fs:
            jobs_list.append((cell + '~twin', [os.path.join(cd, 'Twin.scala')]))
    with concurrent.futures.ThreadPoolExecutor(jobs) as ex:
        got = list(ex.map(lambda kv: capture(sc, sl, work, kv[0], kv[1]), jobs_list))
        got += list(ex.map(lambda kv: capture_desc(sc, sl, work, kv[0], kv[1]), descs))
    if only:
        # The cells the prefixes leave out keep their lines.
        keys = {g.split('\t')[0] for g in got}
        path = os.path.join(HERE, 'expected.tsv')
        old = [l.rstrip('\n') for l in open(path) if not l.startswith('#') and l.strip()] if os.path.exists(path) else []
        got += [l for l in old if l.split('\t')[0] not in keys]
    head = '# tests/default-getters/capture.py writes this file: scalac 3.8.4\'s line of each cell of tests/default-getters/gen.py and of its member twin.\n'
    open(os.path.join(HERE, 'expected.tsv'), 'w').write(head + ''.join(g + '\n' for g in sorted(got)))
    kinds = {}
    for g in got:
        k = g.split('\t')[1].split(' ')[0]
        kinds[k] = kinds.get(k, 0) + 1
    print(f'default-getters capture: {len(got)} lines {kinds}')


if __name__ == '__main__':
    main()
