#!/usr/bin/env python3
"""tests/default-getters/run.py <teq> <scala-library jar> <out dir> [--only <prefix>]... [--jobs N] [--update|--add]:
the legs of every cell of tests/default-getters/gen.py against scalac 3.8.4's lines (tests/default-getters/expected.tsv,
captured by capture.py), each bounded (a teq build or check 120 s, a run 60 s):
  check    `teq compiler check`: accepted where scalac runs, or refused with scalac's first error (line, column, the
           message's first line);
  js, interp, jvm   built and run (node; teq's interpreter; java over scala-library): scalac's line;
  workers  the JavaScript built at 1, 2 and 8 threads: the three outputs byte-equal (or the three refusals alike);
  split-js, split-interp, split-jvm   a split cell's library built to products (JavaScript and the interpreter's by
           `compiler check --products`, the JVM's by `--target jvm --products`), its client against them, run;
  desc     the two-module witness's library's JVM products: its default getters' descriptors (desc.py), scalac's
           class files' (a local class's getters are named by teq's own design: no such leg).
THE ORACLE of a cell is scalac's line of its program, with ONE exception: a cell of the ENUMERATED lifted-override
family (gen.py's `lifted` mark, by its rule as coded: an A-typed call, A's getter reading the block's val, a winner
that is a class or a trait whose getter reads the var or nothing, which scalac's LambdaLift lifts with another
signature so that the override is lost; and its member form, a local trait's method overridden by an anonymous class) whose program and member twin both run and print
differently is judged by the TWIN's line, a class
of its own ("scalac's lifted getters lose the override: judged by the member twin"), never counted as scalac's; such
a cell holding state the twin moves (gen.py's `moves`: a captured var, a lazy or by-name default, a forward
reference) is not judged at all, named, since neither line is its answer. Nothing else is ever judged by a twin: not a
repeated invocation whose twin prints otherwise, not a local refusal whose twin compiles. A cell whose scalac line is a
crash or a timeout has no expectation: skipped, named; but one ENUMERATED second family, gen.py's `diamond` mark
(diamond.py's structural rule: a class whose chain declares no getter, under two or more local traits declaring a lifted
getter of which neither derives from the other, where scalac's JVM throws IncompatibleClassChangeError), is judged by
its member twin where the twin runs, a class of its own too ("scalac's lifted getters get no forwarder").
A leg is `<cell>.<leg>` in tests/pending/default-getters.txt (tests/support/pending.py): the run fails where a leg that
is not its oracle's is unlisted, or a listed one now is. The oracle's and the judge's self-tests of planted records run
first. Writes under <out dir> alone; the last line gives the counts."""
import concurrent.futures, hashlib, os, shutil, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(REPO, 'tests', 'right-assoc'))
import lines  # noqa: E402

TWIN_CLASS = "scalac's lifted getters lose the override: judged by the member twin"
TWIN2_CLASS = "scalac's lifted getters get no forwarder: judged by the member twin"


def bounded(cmd, cwd, timeout, env=None):
    try:
        r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout, env=env)
        return r.returncode, r.stdout, r.stderr
    except subprocess.TimeoutExpired:
        return 'timeout', '', ''


def body(line):
    """A line's kind and text, its cell's name dropped."""
    return line.split('\t', 1)[1] if '\t' in line else line


def oracle(marks, local, twin):
    """(expected kind-and-text, class) of a cell: class 'scalac', 'twin', or 'unjudged: <why>'."""
    if local is None:
        return None, 'unjudged: no captured line'
    lk = local.split('\t')[0]
    # The second family (diamond.py's rule, gen.py's `diamond` mark): scalac's own run throws where the twin runs.
    if 'diamond' in marks and lk.startswith('crash') and twin is not None and twin.split('\t')[0] == 'run':
        if 'moves' in marks:
            return None, 'unjudged: an unforwarded-diamond cell whose twin moves state'
        return twin, 'twin2'
    if lk not in ('run', 'refused'):
        return None, f'unjudged: scalac\'s own run is `{lk}`'
    if 'lifted' in marks and twin is not None and lk == 'run' and twin.split('\t')[0] == 'run' and local != twin:
        if 'moves' in marks:
            return None, 'unjudged: a lifted-override cell whose twin moves state'
        return twin, 'twin'
    return local, 'scalac'


def judge_leg(leg, exp, got):
    """Whether teq's line `got` is the expectation `exp` (both kind-and-text)."""
    if leg == 'check':
        return got.split('\t')[0] == 'run' if exp.split('\t')[0] == 'run' else got == exp
    return got == exp


def self_test():
    bad = []
    run12, run11 = 'run\t1 | 2', 'run\t1 | 1'
    planted = [
        ('lifted, twin differs', ('twin,lifted', 'run\t1103', 'run\t3103'), ('run\t3103', 'twin')),
        ('lifted and moving state', ('twin,lifted,moves', 'run\t1102', 'run\t2152'), (None, 'unjudged: a lifted-override cell whose twin moves state')),
        ('a repeated invocation\'s invented exception', ('twin,moves', run11, run12), (run11, 'scalac')),
        ('a local refusal beside a compiling twin', ('twin,moves', 'refused\t9:7: forward reference', 'run\t0'), ('refused\t9:7: forward reference', 'scalac')),
        ('a lifted cell refused locally', ('twin,lifted', 'refused\t3:1: x', 'run\t1'), ('refused\t3:1: x', 'scalac')),
        ('lifted, twin agrees', ('twin,lifted', 'run\t7', 'run\t7'), ('run\t7', 'scalac')),
        ('scalac crashed', ('-', 'crash 1\t', None), (None, "unjudged: scalac's own run is `crash 1`")),
        ('the second family, scalac throwing', ('twin,diamond', 'crash 1\t', 'run\t212'), ('run\t212', 'twin2')),
        ('the second family moving state', ('twin,diamond,moves', 'crash 1\t', 'run\t212'), (None, 'unjudged: an unforwarded-diamond cell whose twin moves state')),
        ('a crash of another cause beside a running twin', ('twin', 'crash 1\t', 'run\t30'), (None, "unjudged: scalac's own run is `crash 1`")),
        ('no captured line', ('-', None, None), (None, 'unjudged: no captured line')),
    ]
    for name, args, want in planted:
        if oracle(*args) != want:
            bad.append(f'oracle: {name}: {oracle(*args)} for {want}')
    legs = [
        (('check', 'run\t5', 'run\t'), True), (('check', 'run\t5', 'refused\t1:1: x'), False),
        (('check', 'refused\t2:3: m', 'refused\t2:3: m'), True), (('check', 'refused\t2:3: m', 'refused\t2:4: m'), False),
        (('js', 'run\t5', 'run\t5'), True), (('js', 'run\t5', 'run\t6'), False), (('jvm', 'run\t5', 'crash 1\t'), False),
    ]
    for (leg, e, g), want in legs:
        if judge_leg(leg, e, g) != want:
            bad.append(f'judge: {leg} {e!r} {g!r}')
    if bad:
        sys.exit('default-getters: SELF-TEST FAILED: ' + '; '.join(bad))
    return len(planted) + len(legs)


def legs_of(teq, sl, cell, src, work, split):
    os.makedirs(work, exist_ok=True)
    names = sorted(f for f in os.listdir(src) if f.endswith('.scala') and f != 'Twin.scala')
    for f in names:
        shutil.copy(os.path.join(src, f), os.path.join(work, f))
    res = {}

    def line(c, o, e):
        """teq's result as a line: a run's output (exit 0), a refusal's first error, or a crash or a timeout."""
        if c == 0 or c == 'timeout':
            return body(lines.run_line(cell, o, c))
        if ': error: ' in o + e:
            return body(lines.refused_line(cell, o + e))
        return body(lines.run_line(cell, o, c))
    c, o, e = bounded([teq, 'compiler', 'check'] + names, work, 120)
    res['check'] = 'run\t' if c == 0 else line(c, o, e)
    outs = []
    for th in ('1', '2', '8'):
        c, o, e = bounded([teq, 'compiler', 'build'] + names + ['-o', f'a{th}.js', '--threads', th], work, 120)
        outs.append(('ok', hashlib.sha256(open(os.path.join(work, f'a{th}.js'), 'rb').read()).hexdigest()) if c == 0 else ('refused', line(c, o, e)))
        if th == '1':
            js_built = (c, o, e)
    res['workers'] = 'workers\tequal' if len(set(outs)) == 1 else 'workers\tapart: ' + ' / '.join(x[1][:16] for x in outs)
    c, o, e = js_built
    if c == 0:
        c, o, e = bounded(['node', 'a1.js'], work, 60)
    res['js'] = line(c, o, e)
    c, o, e = bounded([teq, 'interp'] + names, work, 120)
    res['interp'] = line(c, o, e)
    c, o, e = bounded([teq, 'compiler', 'build'] + names + ['--target', 'jvm', '--classpath', sl, '-o', 'jv'], work, 120)
    if c == 0:
        c, o, e = bounded(['java', '-XX:-UsePerfData', '-Xss512m', '-cp', os.path.join(work, 'jv') + ':' + sl, 'TeqMain'], work, 60)
    res['jvm'] = line(c, o, e)
    if split:
        lib = [f for f in names if f == 'Lib.scala']
        client = [f for f in names if f != 'Lib.scala']
        c, o, e = bounded([teq, 'compiler', 'check', '--products', 'ap', '--sourceroot', work] + lib, work, 120)
        if c != 0:
            res['split-js'] = res['split-interp'] = 'library\t' + line(c, o, e)
        else:
            c2, o2, e2 = bounded([teq, 'compiler', 'build'] + client + ['--sourceroot', work, '--classpath', 'ap', '-o', 'b.js'], work, 120)
            if c2 == 0:
                c2, o2, e2 = bounded(['node', 'b.js'], work, 60)
            res['split-js'] = line(c2, o2, e2)
            c2, o2, e2 = bounded([teq, 'interp'] + client + ['--sourceroot', work, '--classpath', 'ap'], work, 120)
            res['split-interp'] = line(c2, o2, e2)
        c, o, e = bounded([teq, 'compiler', 'build', '--target', 'jvm', '--products', 'jp', '--sourceroot', work, '--classpath', sl] + lib, work, 120)
        if c != 0:
            res['split-jvm'] = 'library\t' + line(c, o, e)
        else:
            c, o, e = bounded([teq, 'compiler', 'build'] + client + ['--target', 'jvm', '--sourceroot', work, '--classpath', 'jp:' + sl, '-o', 'cl'], work, 120)
            if c == 0:
                c, o, e = bounded(['java', '-XX:-UsePerfData', '-Xss512m', '-cp', ':'.join([os.path.join(work, 'cl'), os.path.join(work, 'jp'), sl]), 'TeqMain'], work, 60)
            res['split-jvm'] = line(c, o, e)
        # The default getters' descriptors of the library's JVM products, scalac's (desc.py).
        jp = os.path.join(work, 'jp')
        if not cell.startswith('SPLIT-witness-'):
            return res  # a local class's names are teq's own design: no descriptor leg
        classes = sorted(os.path.join(dp, f) for dp, _, fs in os.walk(jp) for f in fs
                         if f.endswith('.class') and os.sep + 'scala' + os.sep not in dp + os.sep) if os.path.isdir(jp) else []
        if classes:
            _, jout, _ = bounded(['javap', '-s', '-p'] + classes, work, 120)
            r = subprocess.run([sys.executable, os.path.join(HERE, 'desc.py')], input=jout, capture_output=True, text=True)
            res['desc'] = 'desc\t' + r.stdout.strip()
        else:
            res['desc'] = 'desc\tno library products'
    return res


def main():
    a = sys.argv[1:]
    if len(a) < 3:
        sys.exit(__doc__)
    teq, sl, out = os.path.abspath(a[0]), a[1], os.path.abspath(a[2])
    only = [a[i + 1] for i, x in enumerate(a) if x == '--only']
    jobs = int(a[a.index('--jobs') + 1]) if '--jobs' in a else 8
    mode = [x for x in a[3:] if x in ('--update', '--add')]
    n = self_test()
    print(f'default-getters: self-test: {n} of {n} planted records judged as planted')
    shutil.rmtree(out, ignore_errors=True)
    os.makedirs(out)
    table = os.path.join(out, 'table')
    r = subprocess.run([sys.executable, os.path.join(HERE, 'gen.py'), table], capture_output=True, text=True)
    if r.returncode:
        print(r.stdout + r.stderr)
        sys.exit(1)
    print('default-getters: ' + r.stdout.strip())
    exp = {}
    for l in open(os.path.join(HERE, 'expected.tsv')):
        if l.startswith('#') or not l.strip():
            continue
        k, rest = l.rstrip('\n').split('\t', 1)
        exp[k] = rest
    cells = []
    for l in open(os.path.join(table, 'cells.tsv')):
        c, fam, marks = l.rstrip('\n').split('\t')
        if only and not any(c.startswith(p) for p in only):
            continue
        cells.append((c, fam, marks))
    plan = {c: oracle(marks, exp.get(c), exp.get(c + '~twin')) for c, _, marks in cells}
    twin_judged = sorted(c for c, (_, k) in plan.items() if k == 'twin')
    twin2_judged = sorted(c for c, (_, k) in plan.items() if k == 'twin2')
    unjudged = sorted((c, k) for c, (_, k) in plan.items() if k.startswith('unjudged'))
    print(f'default-getters: {len(twin_judged)} cells judged by their member twin ({TWIN_CLASS}): ' + ' '.join(twin_judged))
    print(f'default-getters: {len(twin2_judged)} cells judged by their member twin ({TWIN2_CLASS}): ' + ' '.join(twin2_judged))
    print(f'default-getters: {len(unjudged)} cells not judged: ' + '; '.join(f'{c} ({k[10:]})' for c, k in unjudged))

    def one(cell):
        c, fam, marks = cell
        e, k = plan[c]
        if e is None:
            return c, None, k
        return c, legs_of(teq, sl, c, os.path.join(table, 'cells', c), os.path.join(out, 'work', c), 'split' in marks), k
    with concurrent.futures.ThreadPoolExecutor(jobs) as ex:
        results = list(ex.map(one, cells))
    lines_out, passed, failed, by_class = [], 0, 0, {}
    for c, res, k in results:
        if res is None:
            lines_out.append(f'skip {c}\t{k}')
            continue
        e = plan[c][0]
        for leg, got in sorted(res.items()):
            if leg == 'desc':
                want = exp.get(c + '~desc')
                if want is None:
                    lines_out.append(f'skip {c}.desc\tno captured descriptors')
                    continue
                ok = got == want
            else:
                ok = got == 'workers\tequal' if leg == 'workers' else judge_leg(leg, e, got)
            if ok:
                lines_out.append(f'pass {c}.{leg}')
                passed += 1
                by_class[k] = by_class.get(k, 0) + 1
            else:
                lines_out.append(f'fail {c}.{leg}\t' + got.replace('\t', ' ').replace('\n', ' ')[:200])
                failed += 1
    rp = os.path.join(out, 'results.txt')
    open(rp, 'w').write(''.join(l + '\n' for l in lines_out))
    r = subprocess.run([sys.executable, os.path.join(REPO, 'tests', 'support', 'pending.py'), 'default-getters', rp] + mode + (['--partial'] if only else []))
    twins, twins2 = by_class.get('twin', 0), by_class.get('twin2', 0)
    print(f'default-getters: {len(cells)} cells, {passed} legs pass ({twins} of them {TWIN_CLASS}; {twins2} {TWIN2_CLASS}), {failed} fail'
          + (' as tests/pending/default-getters.txt lists' if r.returncode == 0 else ', NOT as listed'))
    sys.exit(r.returncode)


if __name__ == '__main__':
    main()
