#!/usr/bin/env python3
"""tests/java-lang-names/equal.py <teq> <teq P> <teq O> <out> [jobs]: two equalities of records over the cells of
cells.tsv and the programs of tests/cases. A program whose source writes `Predef.` as a qualifier (`Predef.String`,
`scala.Predef.Set`, `import scala.Predef.x`; a comment's text does not count) is in P, every other in O. Every program
of P is to give with <teq> the record it gives with <teq P>, every program of O the record it gives with <teq O>; and,
as the control that a difference is seen, <teq O> against <teq P> over P is to differ in at least one program.
A record is a program's four legs, as the suites run them: JavaScript (`compiler build -o`: diagnostics, exit, the
file's digest; node's output and exit), JavaScript over scala-library (the same with `--std=scala-library`), the
interpreter (`interp`: output and exit) and the JVM (`compiler build --target jvm`: diagnostics, exit, every class
file's digest; `java TeqMain`'s output and exit). A case's `// teq:` flags and `// jars:` jars are taken as
tests/run.sh takes them. Every binary works in the same directory (<out>/w/<program>), and the directory is masked in
what is compared. -> <out>/<tag>.json per binary and side, <out>/result.txt with each side's reach (how many programs
each leg built and ran); the exit code is 0 when both equalities hold and the control differs. Without node, java or
scala-library's jar, or with an <out> that already holds files, it does not run."""
import concurrent.futures, glob, hashlib, json, os, re, shutil, subprocess, sys
TEQ, TEQ_P, TEQ_O, OUT = sys.argv[1:5]
JOBS = int(sys.argv[5]) if len(sys.argv) > 5 else os.cpu_count() or 4
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
QUALIFIER = re.compile(r'(?<![\w$])Predef\s*\.')
def run(cmd, timeout=120):
    try:
        r = subprocess.run(cmd, capture_output=True, timeout=timeout, cwd=ROOT)
        return r.returncode, (r.stdout + r.stderr).decode('utf-8', 'replace')
    except subprocess.TimeoutExpired:
        return 'timeout', ''
def digest(paths):
    h = hashlib.sha256()
    for p in sorted(paths):
        h.update(os.path.basename(p).encode() + b'\0' + open(p, 'rb').read())
    return h.hexdigest()[:16]
# Without node, java or scala-library's jar a leg fails alike on every binary and the equalities would hold by it:
# refused, as is an <out> that already holds a run's records.
SL = subprocess.run(['bash', '-c', '. tests/support/jars.sh; jar_of scala-library'], capture_output=True, text=True, cwd=ROOT).stdout.strip()
missing = [n for n in ('node', 'java') if not shutil.which(n)] + ([] if os.path.isfile(SL) else ['scala-library 3.8.4'])
if missing:
    sys.exit('equal.py: missing ' + ', '.join(missing))
if os.path.isdir(OUT) and os.listdir(OUT):
    sys.exit(f'equal.py: {OUT} is not empty')
def writes_qualifier(text):
    return any(QUALIFIER.search(l.split('//', 1)[0]) for l in text.splitlines())
JARS = {}
def jar_of(name):
    if name not in JARS:
        r = subprocess.run(['bash', '-c', f'. tests/support/jars.sh; jar_of {name}'], capture_output=True, text=True, cwd=ROOT)
        JARS[name] = r.stdout.strip()
    return JARS[name]
# the programs: each cell as a file of its own, each case of tests/cases (a file, or a directory of files)
programs = []
for l in open(os.path.join(ROOT, 'tests/java-lang-names/cells.tsv')):
    if l.startswith('#'):
        continue
    name, pos, src, _ = l.rstrip('\n').split('\t')
    src = src.replace('\\n', '\n').replace('\\\\', '\\')
    programs.append({'key': f'cell/{name}__{pos}', 'src': src, 'flags': [], 'jars': []})
for p in sorted(glob.glob(os.path.join(ROOT, 'tests/cases/*.scala')) + [d for d in glob.glob(os.path.join(ROOT, 'tests/cases/*')) if os.path.isdir(d)]):
    files = sorted(glob.glob(os.path.join(p, '**/*.scala'), recursive=True)) if os.path.isdir(p) else [p]
    if not files:
        continue
    text = '\n'.join(open(f, errors='replace').read() for f in files)
    flags = next((m.group(1).split() for m in re.finditer(r'^// teq: (.*)$', text, re.M)), [])
    jars = next((m.group(1).split() for m in re.finditer(r'^// jars: (.*)$', text, re.M)), [])
    programs.append({'key': 'case/' + os.path.basename(p).removesuffix('.scala'), 'path': p, 'text': text, 'flags': flags, 'jars': jars})
for p in programs:
    p['P'] = writes_qualifier(p.get('src') or p['text'])
# EQUAL_ONLY: a regular expression a program's key (`cell/<cell>`, `case/<case>`) is to match, for a run over a few
if os.environ.get('EQUAL_ONLY'):
    programs = [p for p in programs if re.search(os.environ['EQUAL_ONLY'], p['key'])]
def record(teq, p):
    """The program's four legs with the binary <teq>, <teq P> or <teq O>."""
    b = TEQ_BIN[teq]
    w = os.path.join(OUT, 'w', p['key'].replace('/', '__'))
    shutil.rmtree(w, ignore_errors=True)
    os.makedirs(w)
    if 'src' in p:
        src = os.path.join(w, 'Main.scala')
        open(src, 'w').write(p['src'])
    elif os.path.isdir(p['path']):
        src = os.path.join(w, os.path.basename(p['path']))
        shutil.copytree(p['path'], src)
    else:
        src = os.path.join(w, os.path.basename(p['path']))
        shutil.copy(p['path'], src)
    mask = lambda t: t.replace(w, '<W>')
    jars = ':'.join(jar_of(j) for j in p['jars'])
    flags = p['flags'] + (['--classpath', jars] if jars else [])
    r = {}
    for leg, extra in (('js', []), ('js-sl', ['--std=scala-library'])):
        out = os.path.join(w, leg + '.js')
        e, t = run([b, 'compiler', 'build', src, '-o', out] + extra + flags)
        built = e == 0 and os.path.isfile(out)
        r[leg] = [e, mask(t), digest([out]) if built else None] + (list(run(['node', out], 60)) if built else [])
    r['interp'] = list(run([b, 'interp', src] + flags))
    r['interp'][1] = mask(r['interp'][1])
    jv, cp = os.path.join(w, 'jvm'), ':'.join(x for x in (SL, jars) if x)
    e, t = run([b, 'compiler', 'build', src, '--target', 'jvm', '-o', jv, '--classpath', cp] + p['flags'])
    r['jvm'] = [e, mask(t), digest(glob.glob(os.path.join(jv, '**/*.class'), recursive=True)) if e == 0 else None]
    if e == 0:
        e, t = run(['java', '-XX:-UsePerfData', '-cp', f'{jv}:{cp}', 'TeqMain'], 60)
        r['jvm'] += [e, mask(t)]
    for leg in ('js', 'js-sl'):
        if len(r[leg]) > 3:
            r[leg][4] = mask(r[leg][4])
    shutil.rmtree(w, ignore_errors=True)
    return r
TEQ_BIN = {'teq': TEQ, 'P': TEQ_P, 'O': TEQ_O}
RECORDS = {}
def records(teq, which):
    path = os.path.join(OUT, f'{teq}-{which}.json')
    if (teq, which) in RECORDS:
        return RECORDS[teq, which]
    sel = [p for p in programs if p['P'] == (which == 'P')]
    with concurrent.futures.ThreadPoolExecutor(JOBS) as ex:
        recs = dict(zip([p['key'] for p in sel], ex.map(lambda p: record(teq, p), sel)))
    json.dump(recs, open(path, 'w'), indent=0, sort_keys=True)
    RECORDS[teq, which] = recs
    return recs
# What differs between two runs of one binary is not compared: a line of the cost report a failed build prints (its
# times in ms, how many files came from the jar cache) and a run of eight or more digits (a temporary file's name).
COST_LINE = re.compile(r'\d ms\b|jar cache')
def canon(rec):
    def text(x):
        if not isinstance(x, str):
            return x
        return '\n'.join(re.sub(r'\d{8,}', '<n>', l) for l in x.splitlines() if not COST_LINE.search(l))
    return {leg: [text(x) for x in v] for leg, v in rec.items()}
os.makedirs(OUT, exist_ok=True)
lines, ok = [], True
for which, base in (('P', 'P'), ('O', 'O')):
    a, b = records('teq', which), records(base, which)
    differ = sorted(k for k in a if canon(a[k]) != canon(b[k]))
    lines.append(f'{which}: {len(a)} programs; equal to <teq {base}> {len(a) - len(differ)}, differ {len(differ)}')
    lines += [f'  differs {k}: ' + ', '.join(l for l in a[k] if canon(a[k])[l] != canon(b[k])[l]) for k in differ]
    ok = ok and not differ
# the reach: of each side's programs, how many each leg built (exit 0) and ran (the run's exit 0) with <teq>
for which in ('P', 'O'):
    recs = records('teq', which).values()
    lines.append(f'reach {which}: ' + ', '.join(f"{leg} built {sum(1 for r in recs if r[leg][0] == 0)}" + (f" ran {sum(1 for r in recs if len(r[leg]) > 3 and r[leg][3] == 0)}" if leg != 'interp' else '') for leg in ('js', 'js-sl', 'interp', 'jvm')))
c, d = records('O', 'P'), records('P', 'P')
control = sorted(k for k in c if canon(c[k]) != canon(d[k]))
lines.append(f'control: <teq O> against <teq P> over P: differ {len(control)} of {len(c)}')
lines += [f'  differs {k}: ' + ', '.join(l for l in c[k] if canon(c[k])[l] != canon(d[k])[l]) for k in control]
ok = ok and bool(control)
open(os.path.join(OUT, 'result.txt'), 'w').write('\n'.join(lines) + '\n')
print('\n'.join(lines))
sys.exit(0 if ok else 1)
