#!/usr/bin/env python3
"""The conversion search's matrix (generate.py beside this file): programs whose output, or whose refusal, says which
implicit conversion of several a selection or an expected type takes. scalac 3.8.4's answer for each is the oracle,
scalac.jsonl: a record a line, {"id": "<set>/<name>", "digest": <the sources' sha256>, "exit": 0 | 1, "stdout": <its
whole output>}. Each program is built by teq and run on a leg (js: the lean std on node; sl: scala-library on node;
jvm: the JVM back end on java; interp: the interpreter); a job is a (program, leg).

  run.py [--tenth | --whole] [--sets a,b] [--results DIR]   build and run the jobs, then check them
  run.py --check --results DIR [--tenth | --whole]          check the records a run left in DIR
  run.py --whole --update                                   also write the board's rows, tests/board/suites/
                                                            conversion-matrix-<set>.tsv, from the run
  run.py --record [--sets a,b]                              rewrite scalac.jsonl by scalac 3.8.4 (Batch.scala): every
                                                            program, or the sets named, the others' records kept

Without --tenth or --whole every program runs on js; --tenth takes every tenth program of each set (the gate's line),
--whole every program on the four legs (after a landing).

The check fails, with a line each, where:
- the generated programs and the oracle differ: an id or a digest, a duplicate, a malformed record;
- a job has no record, an empty or malformed one, or two; a record answers no job; the records name two binaries
  (each names the version of the teq that wrote it);
- a job's record says the infrastructure failed (a missing executable or jar, a failed worker), timed out, died of a
  signal or panicked: none of these is a refusal, and a run's failure is not one either;
- a job is not right (teq's whole stdout and exit equal scalac's, or teq refuses where scalac refuses: a compile
  error, exit 1 with an `error found` summary) and known.tsv does not hold it with the outcome observed;
- known.tsv holds a malformed line, a duplicate, an id or leg the programs lack, a job that is right now, or an
  outcome other than the one observed.
Where all hold it exits 0, its one line the count of jobs and of the known departures among them. Before anything the checker is run on stand-ins of each of these, and
a stand-in it lets pass fails the line (`selftest`).
known.tsv: id, leg, owner, cause, observed (the outcome as the check prints it), tab-separated, `#` comments.
TEQ names the binary (target/release/teq), SL scala-library's jar (the jvm leg), JOBS the jobs at once (default 6)."""
import argparse, hashlib, json, os, shutil, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
LEGS = ('js', 'interp', 'jvm', 'sl')


def digest(d):
    h = hashlib.sha256()
    for fn in sorted(os.listdir(d)):
        if fn.endswith('.scala'):
            h.update(fn.encode() + b'\0')
            with open(os.path.join(d, fn), 'rb') as f:
                h.update(f.read() + b'\0')
    return h.hexdigest()


def generate(out):
    subprocess.run([sys.executable, os.path.join(HERE, 'generate.py'), out], check=True, stdout=subprocess.DEVNULL)
    ids = []
    for s in sorted(os.listdir(out)):
        for n in sorted(os.listdir(os.path.join(out, s))):
            ids.append(f'{s}/{n}')
    return ids


def load_oracle(path, problems):
    oracle = {}
    with open(path) as f:
        for k, line in enumerate(f, 1):
            try:
                r = json.loads(line)
                ok = isinstance(r, dict) and isinstance(r.get('id'), str) and isinstance(r.get('digest'), str) and r.get('exit') in (0, 1) and isinstance(r.get('stdout'), str)
            except ValueError:
                ok = False
            if not ok:
                problems.append(f'FAIL scalac.jsonl line {k}: malformed record')
                continue
            if r['id'] in oracle:
                problems.append(f'FAIL scalac.jsonl: {r["id"]} recorded twice')
            oracle[r['id']] = r
    return oracle


def selected(ids, mode, sets):
    ids = [i for i in ids if not sets or i.split('/')[0] in sets]
    if mode == 'tenth':
        by_set = {}
        for i in ids:
            by_set.setdefault(i.split('/')[0], []).append(i)
        ids = [i for s in sorted(by_set) for k, i in enumerate(by_set[s]) if k % 10 == 0]
    return ids


def jobs_of(ids, mode):
    legs = LEGS if mode == 'whole' else ('js',)
    return [(i, leg) for i in ids for leg in legs]


def refused(text):
    """A compile error's report: its summary line (`1 error found`, `N errors found`)."""
    return any(l.endswith(' error found') or l.endswith(' errors found') for l in text.splitlines())


def run_job(teq, version, progs, results, sl, job):
    i, leg = job
    d = os.path.join(progs, i)
    w = os.path.join(results, 'work', i.replace('/', '__') + '.' + leg)
    os.makedirs(w, exist_ok=True)
    srcs = sorted(os.path.join(d, f) for f in os.listdir(d) if f.endswith('.scala'))
    rec = {'id': i, 'leg': leg, 'binary': version}

    def done(**k):
        rec.update(k)
        with open(os.path.join(results, 'records', i.replace('/', '__') + '.' + leg + '.json'), 'w') as f:
            json.dump(rec, f)
        # What the job built is not kept: its record holds what the check reads.
        shutil.rmtree(w, ignore_errors=True)

    def compile_(args):
        try:
            p = subprocess.run([teq] + args, capture_output=True, text=True, timeout=60)
        except subprocess.TimeoutExpired:
            return 'timeout', None
        except OSError as e:
            return 'infra', str(e)
        if 'panicked' in p.stderr or 'panicked' in p.stdout:
            return 'panic', None
        if p.returncode < 0:
            return 'signal', None
        if p.returncode == 0:
            return 'built', p
        if p.returncode == 1 and refused(p.stdout + p.stderr):
            return 'refused', None
        return 'infra', f'teq exit {p.returncode}'

    def execute(cmd):
        try:
            p = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        except subprocess.TimeoutExpired:
            return done(kind='timeout')
        except OSError as e:
            return done(kind='infra', why=str(e))
        if p.returncode < 0:
            return done(kind='signal')
        done(kind='ran', exit=p.returncode, stdout=p.stdout)

    if leg in ('js', 'sl'):
        std = 'lean' if leg == 'js' else 'scala-library'
        out = os.path.join(w, 'a.js')
        k, p = compile_(['compiler', 'build'] + srcs + ['--std', std, '--threads', '1', '--no-cache', '-o', out])
        if k != 'built':
            return done(kind=k, why=p) if k == 'infra' else done(kind=k)
        return execute(['node', out])
    if leg == 'jvm':
        out = os.path.join(w, 'a.jar')
        k, p = compile_(['compiler', 'build'] + srcs + ['--target', 'jvm', '--classpath', sl, '--threads', '1', '--no-cache', '-o', out])
        if k != 'built':
            return done(kind=k, why=p) if k == 'infra' else done(kind=k)
        return execute(['java', '-cp', out + os.pathsep + sl, 'TeqMain'])
    # The interpreter builds and runs at once: a compile error is its refusal, any other failure its run's.
    try:
        p = subprocess.run([teq, 'interp'] + srcs + ['--threads', '1', '--no-cache'], capture_output=True, text=True, timeout=90)
    except subprocess.TimeoutExpired:
        return done(kind='timeout')
    except OSError as e:
        return done(kind='infra', why=str(e))
    if 'panicked' in p.stderr:
        return done(kind='panic')
    if p.returncode < 0:
        return done(kind='signal')
    if p.returncode == 1 and refused(p.stderr):
        return done(kind='refused')
    done(kind='ran', exit=p.returncode, stdout=p.stdout)


def outcome(rec):
    if rec['kind'] == 'ran':
        return f'ran exit {rec["exit"]} stdout {hashlib.sha256(rec["stdout"].encode()).hexdigest()[:16]}'
    return rec['kind']


def is_right(rec, o):
    return (o['exit'] == 1 and rec['kind'] == 'refused') or (o['exit'] == 0 and rec['kind'] == 'ran' and rec['exit'] == 0 and rec['stdout'] == o['stdout'])


def load_records(results, problems):
    recs = {}
    rdir = os.path.join(results, 'records')
    names = sorted(os.listdir(rdir)) if os.path.isdir(rdir) else []
    for fn in names:
        p = os.path.join(rdir, fn)
        try:
            with open(p) as f:
                r = json.load(f)
            key = (r['id'], r['leg'])
            if r['kind'] not in ('ran', 'refused', 'infra', 'timeout', 'signal', 'panic') or (r['kind'] == 'ran' and not (isinstance(r.get('exit'), int) and isinstance(r.get('stdout'), str))):
                raise ValueError
        except (ValueError, KeyError, TypeError, OSError):
            problems.append(f'FAIL record {fn}: empty or malformed')
            continue
        if key in recs:
            problems.append(f'FAIL {key[0]} {key[1]}: two records')
        recs[key] = r
    return recs


def load_known(known_path, digests, problems):
    known = {}
    with open(known_path) as f:
        for k, line in enumerate(f, 1):
            if not line.strip() or line.startswith('#'):
                continue
            parts = line.rstrip('\n').split('\t')
            if len(parts) != 5 or not all(parts):
                problems.append(f'FAIL known.tsv line {k}: not id, leg, owner, cause, observed')
                continue
            key = (parts[0], parts[1])
            if key in known:
                problems.append(f'FAIL known.tsv: {key[0]} {key[1]} listed twice')
            if parts[0] not in digests or parts[1] not in LEGS:
                problems.append(f'FAIL known.tsv: {key[0]} {key[1]} names no program or leg (stale)')
            known[key] = parts
    return known


def check(ids, digests, oracle, results, jobs, known_path, problems):
    # The programs against the oracle, whole.
    if len(set(ids)) != len(ids):
        problems.append('FAIL the generator makes a program twice')
    for i in ids:
        r = oracle.get(i)
        if r is None:
            problems.append(f'FAIL {i}: no scalac record')
        elif r['digest'] != digests[i]:
            problems.append(f'FAIL {i}: its sources are not those scalac answered (digest)')
    for i in oracle:
        if i not in digests:
            problems.append(f'FAIL scalac.jsonl: {i} is no program')
    # The records against the jobs, once each.
    recs = load_records(results, problems)
    binaries = sorted({r.get('binary', '') for r in recs.values()})
    if len(binaries) > 1:
        problems.append(f'FAIL the records name {len(binaries)} binaries: {", ".join(b or "none" for b in binaries)}')
    want = set(jobs)
    for key in jobs:
        if key not in recs:
            problems.append(f'FAIL {key[0]} {key[1]}: no record')
    for key in recs:
        if key not in want:
            problems.append(f'FAIL {key[0]} {key[1]}: a record of no job')
    known = load_known(known_path, digests, problems)
    # Each job's outcome.
    for key in jobs:
        r = recs.get(key)
        if r is None:
            continue
        o = oracle.get(key[0])
        if r['kind'] in ('infra', 'timeout', 'signal', 'panic'):
            problems.append(f'FAIL {key[0]} {key[1]}: {r["kind"]}{": " + r["why"] if r.get("why") else ""}')
            continue
        if o is None:
            continue
        right = is_right(r, o)
        listed = known.get(key)
        if right and listed is not None:
            problems.append(f'FAIL {key[0]} {key[1]}: right now, listed in known.tsv')
        elif not right and listed is None:
            problems.append(f'FAIL {key[0]} {key[1]}: not right ({outcome(r)}), not listed')
        elif not right and listed[4] != outcome(r):
            problems.append(f'FAIL {key[0]} {key[1]}: listed as "{listed[4]}", observed "{outcome(r)}"')
    return recs, known


def selftest():
    """The checker on stand-ins of what it must refuse: each case a world of two programs, its oracle, its records and
    its known list, and the line the check must print. A case it lets pass is a failure of the line."""
    failures = []
    ids = ['s/a', 's/b']
    digests = {'s/a': 'da', 's/b': 'db'}
    good_oracle = {'s/a': {'id': 's/a', 'digest': 'da', 'exit': 0, 'stdout': 'one\ntwo\n'}, 's/b': {'id': 's/b', 'digest': 'db', 'exit': 1, 'stdout': ''}}
    ran = {'id': 's/a', 'leg': 'js', 'kind': 'ran', 'exit': 0, 'stdout': 'one\ntwo\n'}
    ref = {'id': 's/b', 'leg': 'js', 'kind': 'refused'}
    jobs = [('s/a', 'js'), ('s/b', 'js')]

    def case(name, expect, records=None, oracle=None, known='', raw=None, oracle_lines=None, gen_ids=None):
        with tempfile.TemporaryDirectory() as t:
            os.makedirs(os.path.join(t, 'records'))
            for k, r in enumerate(records if records is not None else [ran, ref]):
                with open(os.path.join(t, 'records', f'r{k}.json'), 'w') as f:
                    f.write(r if isinstance(r, str) else json.dumps(r))
            kp = os.path.join(t, 'known.tsv')
            with open(kp, 'w') as f:
                f.write(known)
            problems = []
            if oracle_lines is not None:
                op = os.path.join(t, 'scalac.jsonl')
                with open(op, 'w') as f:
                    f.write(oracle_lines)
                o = load_oracle(op, problems)
            else:
                o = oracle if oracle is not None else good_oracle
            check(gen_ids or ids, digests, o, t, jobs, kp, problems)
            if expect is None:
                if problems:
                    failures.append(f'FAIL the checker\'s own test "{name}": it reports {problems[0]}')
            elif not any(expect in p for p in problems):
                failures.append(f'FAIL the checker\'s own test "{name}": no line saying "{expect}"')

    out_a = hashlib.sha256('one\nTWO\n'.encode()).hexdigest()[:16]
    case('all right', None)
    case('an empty results directory', 's/a js: no record', records=[])
    case('a missing record', 's/b js: no record', records=[ran])
    case('two records of a job', 's/a js: two records', records=[ran, ran, ref])
    case('a record of no job', 's/a jvm: a record of no job', records=[ran, ref, dict(ran, leg='jvm')])
    case('an empty record', 'empty or malformed', records=[ran, ref, ''])
    case('a malformed record', 'empty or malformed', records=[ran, ref, '{"id": "s/a"}'])
    case('records of two binaries', 'the records name 2 binaries', records=[dict(ran, binary='teq 1'), dict(ref, binary='teq 2')])
    case('a timeout', 's/a js: timeout', records=[dict(id='s/a', leg='js', kind='timeout'), ref])
    case('a signal', 's/a js: signal', records=[dict(id='s/a', leg='js', kind='signal'), ref])
    case('a panic', 's/b js: panic', records=[ran, dict(id='s/b', leg='js', kind='panic')])
    case('a missing executable', 's/b js: infra', records=[ran, dict(id='s/b', leg='js', kind='infra', why='no node')])
    case('a run\'s failure where scalac refuses', 's/b js: not right (ran exit 1', records=[ran, dict(id='s/b', leg='js', kind='ran', exit=1, stdout='')])
    case('the first line alone right', f's/a js: not right (ran exit 0 stdout {out_a})', records=[dict(ran, stdout='one\nTWO\n'), ref])
    case('a refusal where scalac builds', 's/a js: not right (refused)', records=[dict(id='s/a', leg='js', kind='refused'), ref])
    case('a digest that is not the sources\'', 's/a: its sources are not those scalac answered', oracle=dict(good_oracle, **{'s/a': dict(good_oracle['s/a'], digest='dx')}))
    case('a program without a record', 's/b: no scalac record', oracle={'s/a': good_oracle['s/a']})
    case('a record of no program', 'scalac.jsonl: s/c is no program', oracle=dict(good_oracle, **{'s/c': dict(good_oracle['s/a'], id='s/c')}))
    lines = ''.join(json.dumps(good_oracle[i]) + '\n' for i in ids)
    case('an oracle record twice', 'scalac.jsonl: s/a recorded twice', oracle_lines=lines + json.dumps(good_oracle['s/a']) + '\n')
    case('a malformed oracle line', 'scalac.jsonl line 3: malformed record', oracle_lines=lines + '{"id": "s/c", "exit": 2}\n')
    case('a program made twice', 'the generator makes a program twice', gen_ids=['s/a', 's/a', 's/b'])
    known_a = 's/a\tjs\towner\tcause\trefused\n'
    case('a known failure', None, records=[dict(id='s/a', leg='js', kind='refused'), ref], known=known_a)
    case('a known failure that is right now', 's/a js: right now, listed in known.tsv', known=known_a)
    case('a known failure that changed', 's/a js: listed as "refused"', records=[dict(ran, stdout='x\n'), ref], known=known_a)
    case('a known line twice', 's/a js listed twice', records=[dict(id='s/a', leg='js', kind='refused'), ref], known=known_a + known_a)
    case('a stale known line', 's/z js names no program or leg', known='s/z\tjs\towner\tcause\trefused\n')
    case('a known line of no leg', 's/a wasm names no program or leg', known='s/a\twasm\towner\tcause\trefused\n')
    case('a malformed known line', 'known.tsv line 1: not id, leg, owner, cause, observed', known='s/a\tjs\trefused\n')
    return failures


def record(sets):
    """scalac.jsonl anew: each set's programs (or the sets named) compiled and run by dotty 3.8.4 in one JVM
    (Batch.scala, built by scala-cli), then a record a program, the other sets' records kept as they are; a program
    scalac's own run dies on stops the record, as it has no answer."""
    if not shutil.which('scala-cli'):
        print('--record: scala-cli not found')
        return 2
    work = tempfile.mkdtemp(prefix='teq-conversion-record.')
    try:
        progs = os.path.join(work, 'progs')
        ids = generate(progs)
        cp = subprocess.run(['scala-cli', '--power', 'compile', '-S', '3.8.4', '--jvm', 'system', '--server=false', '-q', '--workspace',
                             os.path.join(work, 'ws'), os.path.join(HERE, 'Batch.scala'), '--print-class-path'],
                            capture_output=True, text=True, timeout=600)
        if cp.returncode != 0:
            print('--record: Batch.scala did not compile')
            return 2
        chosen = [i for i in ids if not sets or i.split('/')[0] in sets]
        for s in sorted({i.split('/')[0] for i in chosen}):
            listing = os.path.join(work, s + '.dirs')
            with open(listing, 'w') as f:
                f.write(''.join(os.path.join(progs, i) + '\n' for i in ids if i.startswith(s + '/')))
            b = subprocess.run(['java', '-Xmx2g', '-cp', cp.stdout.strip(), 'batch', listing, os.path.join(work, 'scalac', s)], timeout=3000)
            if b.returncode != 0:
                print(f'--record: the batch of {s} failed')
                return 2
        kept = {}
        if sets:
            problems = []
            kept = load_oracle(os.path.join(HERE, 'scalac.jsonl'), problems)
            if problems:
                print('--record: the records kept are not well formed: ' + problems[0])
                return 1
        lines, dead = [], []
        for i in ids:
            s, n = i.split('/')
            if sets and s not in sets:
                if i in kept:
                    lines.append(json.dumps(kept[i]))
                continue
            base = os.path.join(work, 'scalac', s, n)
            with open(base + '.exit') as f:
                code = int(f.read().strip())
            if code not in (0, 1):
                dead.append(i)
                continue
            stdout = open(base + '.out', errors='surrogateescape').read() if code == 0 else ''
            lines.append(json.dumps({'id': i, 'digest': digest(os.path.join(progs, i)), 'exit': code, 'stdout': stdout}))
        for d in dead:
            print(f'--record: scalac dies on {d}')
        if dead:
            return 1
        with open(os.path.join(HERE, 'scalac.jsonl'), 'w') as f:
            f.write('\n'.join(lines) + '\n')
        print(f'recorded {len(lines)} programs')
        return 0
    finally:
        shutil.rmtree(work, ignore_errors=True)


def update_board(ids, oracle, recs, known):
    """The board's rows of a whole run: a file a set, a line a program, failing where a leg is not right, with its
    known owner and cause."""
    by_set = {}
    for i in ids:
        by_set.setdefault(i.split('/')[0], []).append(i)
    for s, names in by_set.items():
        lines = ['# test\tverdict\tthe other ways\tthe first line of the difference']
        for i in names:
            wrong = [leg for leg in LEGS if (i, leg) in recs and not is_right(recs[(i, leg)], oracle[i])]
            if not wrong:
                lines.append(f'{i.split("/", 1)[1]}\tpass\t-\t')
                continue
            k = known.get((i, wrong[0]))
            why = f'{", ".join(wrong)}: {outcome(recs[(i, wrong[0])])}' + (f' (known: {k[2]}: {k[3]})' if k else '')
            lines.append(f'{i.split("/", 1)[1]}\tfail\t-\t{why}')
        with open(os.path.join(REPO, 'tests', 'board', 'suites', f'conversion-matrix-{s}.tsv'), 'w') as f:
            f.write('\n'.join(lines) + '\n')


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--tenth', action='store_true')
    ap.add_argument('--whole', action='store_true')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--record', action='store_true')
    ap.add_argument('--update', action='store_true')
    ap.add_argument('--sets')
    ap.add_argument('--results')
    a = ap.parse_args()
    failures = selftest()
    for f in failures:
        print(f)
    if failures:
        return 1
    if a.record:
        return record(set(a.sets.split(',')) if a.sets else None)
    mode = 'tenth' if a.tenth else 'whole' if a.whole else 'all'
    if a.update and (mode != 'whole' or a.sets):
        print('--update: the board\'s rows come from a whole run of every set')
        return 2
    sets = set(a.sets.split(',')) if a.sets else None
    work = tempfile.mkdtemp(prefix='teq-conversion-matrix.')
    try:
        progs = os.path.join(work, 'progs')
        ids = generate(progs)
        digests = {i: digest(os.path.join(progs, i)) for i in ids}
        problems = []
        oracle = load_oracle(os.path.join(HERE, 'scalac.jsonl'), problems)
        chosen = selected(ids, mode, sets)
        jobs = jobs_of(chosen, mode)
        results = a.results or os.path.join(work, 'results')
        if not a.check:
            teq = os.environ.get('TEQ', os.path.join(REPO, 'target', 'release', 'teq'))
            sl = os.environ.get('SL', '')
            need = [teq if os.path.isfile(teq) else None, shutil.which('node')]
            if mode == 'whole':
                need += [shutil.which('java'), sl if sl and os.path.isfile(sl) else None]
            if None in need:
                print('FAIL a missing executable or jar: the line cannot run (teq, node, java, scala-library)')
                return 1
            version = subprocess.run([teq, '--version'], capture_output=True, text=True, timeout=60).stdout.strip()
            os.makedirs(os.path.join(results, 'records'), exist_ok=True)
            with ThreadPoolExecutor(int(os.environ.get('JOBS', '6'))) as pool:
                list(pool.map(lambda j: run_job(os.path.abspath(teq), version, progs, results, sl, j), jobs))
        recs, known = check(ids, digests, oracle, results, jobs, os.path.join(HERE, 'known.tsv'), problems)
        for p in problems:
            print(p)
        if problems:
            return 1
        if a.update:
            update_board(ids, oracle, recs, known)
        departing = sum(1 for j in jobs if j in known)
        print(f'conversion-matrix: {len(jobs)} jobs of {len(chosen)} programs: {len(jobs) - departing} right, {departing} departing as known.tsv lists')
        return 0
    finally:
        shutil.rmtree(work, ignore_errors=True)


if __name__ == '__main__':
    sys.exit(main())
