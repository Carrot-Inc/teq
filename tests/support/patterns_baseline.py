#!/usr/bin/env python3
"""The extractors' landing A is proved by equality with master: this freezes master's outputs over the corpus and
compares a head's with them.

For every program of tests/cases (a file or a directory, with its `// teq:` flags and `// jars:` class path, as
tests/run.sh, run_jvm.sh and run_interp.sh take them), the baseline records:
- the JavaScript build: its exit code, its diagnostics, and the bytes of its output (sha256);
- the JVM build into a class directory: its exit code, its diagnostics, and the bytes of every class file; and the
  same build typed by four workers (TEQ_THREADS=4 sets the typer's workers whatever the program's size);
- the check build's products (`--products`): the TASTy and the manifest;
- the interpreter: its exit code, its output and its diagnostics.
For every program of tests/errors it records the check's exit code and diagnostics. For every case of tests/modules it
records the whole build as tests/modules.sh makes it (modules a, b and c together, `--products`, made first where the
case has a generate.py): the JVM build's and the check build's exit codes, diagnostics and the bytes of every product.
tests/tasty's programs are recorded the same way: each file of a directory directly under tests/tasty alone, and each
deeper directory's files together (the capture's, hold's and origins' programs), with their `// jars:` class path,
but for the sources TASTY_NOT_MEASURED names; tests/tasty/exec/cases' programs are measured as tests/cases' are.

A test whose jars are not in the coursier cache is recorded as such, on both sides. Every step is bounded; a step past
its bound is recorded as a timeout, which the comparison reports and never counts as equal.

Usage:
  patterns_baseline.py freeze  --teq BIN --dir DIR [--jobs N] [--only REGEX] [--extra DIR]...
  patterns_baseline.py compare --teq BIN --base DIR --dir DIR [--jobs N] [--only REGEX] [--extra DIR]... [--check-plans]
  patterns_baseline.py self-test --teq BIN --dir DIR
Run from the repository's root. `freeze` writes DIR/baseline.json (the binary's version and sha256 with every
record), and never over one that exists. `compare` reads the baseline whole or refuses it, prints its sha256, runs the
head into DIR and prints every difference, exiting 1 on any; the texts are compared through `normalized` (a run's
durations and the like, each in the one step it was seen in), the records keeping them raw. A program that differs is
run again, up to RERUNS times: a difference every run shows is a DIFF; one a rerun clears is UNSTABLE, said, and fails
the comparison unless MASTER_UNSTABLE names the step as one master itself does not settle. `self-test` plants a
JavaScript byte difference, a diagnostic difference, an interpreter output difference, a class-file byte difference, a
module's class-file byte difference, a program's TASTy byte difference, and an output difference in the first run
alone, through a wrapper of BIN, and exits 0 only when the comparison finds each of them and no other (the last as
UNSTABLE, failing), every step having run and the same binary twice comparing equal; over records made for them, it
holds that a timeout on either side, a program or a step on one side alone each fail, that a baseline cut, empty,
without records or short of its units is refused, and that each normalization hides what it names, nothing else and
nowhere else."""
import argparse
import collections
import concurrent.futures
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys

# The Maven tree of the coursier cache, as tests/support/jars.sh finds it.
M2 = pathlib.Path(os.environ.get('COURSIER_CACHE', str(pathlib.Path.home() / 'Library/Caches/Coursier/v1'))) / 'https/repo1.maven.org/maven2'
SCALA_LIBRARY = M2 / 'org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar'
BOUND = {'js': 60, 'jvm': 60, 'interp': 60, 'check': 60, 'module': 180}


def sha(path):
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()


def run(cmd, bound, cwd=None, env=None):
    try:
        p = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=bound, cwd=cwd, env=env)
        return {'rc': p.returncode, 'out': p.stdout.decode('utf-8', 'replace'), 'err': p.stderr.decode('utf-8', 'replace')}
    except subprocess.TimeoutExpired:
        return {'rc': 'timeout', 'out': '', 'err': ''}


def tree_shas(d):
    d = pathlib.Path(d)
    return {str(f.relative_to(d)): sha(f) for f in sorted(d.rglob('*')) if f.is_file()} if d.is_dir() else {}


def first_line(src, prefix):
    files = [src] if src.is_file() else sorted(src.glob('*.scala'))
    for f in files:
        for line in f.read_text(errors='replace').splitlines():
            if line.startswith(prefix):
                return line[len(prefix):].strip()
    return ''


def jars(src):
    p = subprocess.run(['bash', '-c', '. tests/support/jars.sh; jars_of "$1"; printf "%s\\n%s" "$JARS_CP" "$JARS_MISSING"',
                        '_', str(src)], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=60)
    lines = p.stdout.decode().split('\n')
    return (lines[0] if lines else ''), (lines[1].strip() if len(lines) > 1 else '')


# tests/tasty's sources that the driver does not build, each built by its own suite with what the driver does not give
# it; built alone they stop where that is missing, equal on every side, and prove nothing.
ORACLE = "a tool of scalac's: scalac's compiler or its TASTy inspector on the class path"
PRODUCTS = "another program's products or fixtures on the class path, as tests/tasty.sh builds it"
TASTY_NOT_MEASURED = {
    'annotations/Regen.scala': ORACLE, 'fromtasty/Gate.scala': ORACLE, 'inspector/Inspector.scala': ORACLE,
    'inspector/UseInspector.scala': ORACLE, 'positions/Oracle.scala': ORACLE, 'representation/Oracle.scala': ORACLE,
    'representation/closure/plugin/': ORACLE, 'representation/compiler/': ORACLE, 'representation/dotty/': ORACLE,
    'stdshapes/Check.scala': ORACLE, 'stdshapes/Kinds.scala': ORACLE, 'stdshapes/Reflect.scala': ORACLE,
    'annotations/use.scala': PRODUCTS, 'origins/use/': PRODUCTS, 'origins/use/own/': PRODUCTS,
    'places/Inl.scala': PRODUCTS, 'places/Use.scala': PRODUCTS, 'representation/observe.scala': PRODUCTS,
    'representation/probes.scala': PRODUCTS, 'representation/use/': PRODUCTS, 'src/reader.scala': PRODUCTS,
    'src/reader_java.scala': PRODUCTS, 'src/reader_java_v1.scala': PRODUCTS,
    'src/pext_dsl.scala': 'typed with src/pext_inlined.scala as one unit (tests/tasty.sh), which is measured alone',
    'src/unmod.scala': "the Scala.js library on the class path (tests/tasty/README's `--js`)",
    'representation/cc.scala': "the oracle's -source:3.8 and -source:3.9 builds (representation/README.md)",
    'representation/rec/': "the oracle's options, rec/Nulls.scala's -Yexplicit-nulls (representation/README.md)",
}


def case_units(only, extra=()):
    units = []
    for d in extra:
        for p in sorted(pathlib.Path(d).glob('*.scala')):
            units.append(('extra', p.stem, p))
    for p in sorted(pathlib.Path('tests/cases').iterdir()):
        if p.is_dir() or p.suffix == '.scala':
            name = p.stem if p.is_file() else p.name
            units.append(('cases', name, p))
    for p in sorted(pathlib.Path('tests/errors').iterdir()):
        if p.is_dir() or p.suffix == '.scala':
            units.append(('errors', p.stem if p.is_file() else p.name, p))
    for p in sorted(pathlib.Path('tests/modules').iterdir()):
        if (p / 'a').is_dir():
            units.append(('modules', p.name, p))
    root = pathlib.Path('tests/tasty')
    # tests/tasty/exec/cases: programs as tests/cases' are, its files and its directories (tests/tasty-exec.scala).
    for p in sorted((root / 'exec/cases').iterdir()):
        if p.is_dir() or p.suffix == '.scala':
            units.append(('exec', p.stem if p.is_file() else p.name, p))
    for p in sorted(root.rglob('*')):
        rel = p.relative_to(root)
        if any(x.startswith('.') or x == '__pycache__' for x in rel.parts) or rel.parts[:2] == ('exec', 'cases'):
            continue
        name = str(rel) + ('/' if p.is_dir() else '')
        if name in TASTY_NOT_MEASURED:
            continue
        if p.is_file() and p.suffix == '.scala' and len(rel.parts) == 2:
            units.append(('tasty', name, p))
        elif p.is_dir() and len(rel.parts) >= 2 and any(p.glob('*.scala')):
            units.append(('tasty', name, p))
    if only:
        rx = re.compile(only)
        units = [u for u in units if rx.search(u[0] + '/' + u[1])]
    return units


def measure(teq, work, unit):
    kind, name, src = unit
    flags = first_line(src, '// teq:').split()
    cp, missing = jars(src)
    rec = {'kind': kind, 'name': name}
    if missing:
        rec['nojar'] = missing
        return rec
    w = pathlib.Path(os.path.abspath(work)) / kind / name.replace('/', '%')
    if w.exists():
        shutil.rmtree(w)
    w.mkdir(parents=True)
    if kind in ('modules', 'tasty'):
        return measure_products(teq, w, unit, cp, rec)
    if kind == 'errors':
        # As tests/run_errors.sh runs it: `// command: build` builds JavaScript, `// jdk: none` runs without a JDK.
        command = first_line(src, '// command:') or 'check'
        extra = ['--classpath', cp] if cp else []
        if command == 'build':
            extra += ['-o', str(w / 'errors.js')]
        env = None
        if first_line(src, '// jdk:') == 'none':
            env = {k: v for k, v in os.environ.items() if k != 'JAVA_HOME'}
            env['PATH'] = '/nonexistent'
        r = run([os.path.abspath(teq), 'compiler', command, str(src)] + flags + extra, BOUND['check'], env=env)
        rec['check'] = {'rc': r['rc'], 'diag': r['out'] + r['err']}
        return rec
    js_cp = ['--classpath', cp] if cp else []
    r = run([teq, 'compiler', 'build', str(src)] + flags + js_cp + ['--target', 'js', '-o', str(w / 'out.js')], BOUND['js'])
    rec['js'] = {'rc': r['rc'], 'diag': r['out'] + r['err'], 'sha': sha(w / 'out.js') if (w / 'out.js').is_file() else None}
    jvm_cp = str(SCALA_LIBRARY) + (':' + cp if cp else '')
    r = run([teq, 'compiler', 'build', str(src)] + flags + ['--classpath', jvm_cp, '--target', 'jvm', '-o', str(w / 'jvm')],
            BOUND['jvm'])
    classes = {}
    if (w / 'jvm').is_dir():
        for f in sorted((w / 'jvm').rglob('*')):
            if f.is_file():
                classes[str(f.relative_to(w / 'jvm'))] = sha(f)
    rec['jvm'] = {'rc': r['rc'], 'diag': r['out'] + r['err'], 'classes': classes}
    # The same JVM build typed by four workers: the plan's rebuild after the workers' merge (T3).
    env4 = dict(os.environ, TEQ_THREADS='4')
    r = run([teq, 'compiler', 'build', str(src)] + flags + ['--classpath', jvm_cp, '--target', 'jvm', '-o', str(w / 'jvm4')],
            BOUND['jvm'], env=env4)
    rec['jvm4'] = {'rc': r['rc'], 'diag': r['out'] + r['err'], 'classes': tree_shas(w / 'jvm4')}
    # The products a split build reads: the TASTy and the manifest, the plan never among them (T3).
    r = run([teq, 'compiler', 'check', str(src)] + flags + js_cp + ['--products', str(w / 'products')], BOUND['check'])
    rec['products'] = {'rc': r['rc'], 'diag': r['out'] + r['err'], 'classes': tree_shas(w / 'products')}
    r = run([teq, 'interp', str(src)] + flags + js_cp, BOUND['interp'])
    rec['interp'] = {'rc': r['rc'], 'out': r['out'], 'err': r['err']}
    return rec


def measure_products(teq, w, unit, cp, rec):
    """A case of tests/modules built whole, or a program of tests/tasty: the JVM build's and the check build's
    products. The work directories' paths are taken out of the diagnostics, as they differ between two runs."""
    kind, name, src = unit
    root, cwd = src, None
    if kind == 'modules':
        if (src / 'generate.py').is_file():
            # As tests/modules.sh makes such a case: a copy beside its products, completed by its generate.py. It is
            # built from its own directory, so that the manifest's sources name no work directory; its root, and the
            # identity made from it, still do, and are left out of its manifest's record (`masked`).
            root = cwd = w / 'case'
            shutil.copytree(src, root)
            r = run([sys.executable, str(src / 'generate.py'), str(root)], BOUND['module'])
            rec['generate'] = {'rc': r['rc'], 'diag': (r['out'] + r['err']).replace(str(w), '<work>')}
            if r['rc'] != 0:
                return rec
        # tests/modules.sh's `sources`: every .scala file of modules a, b and c, each module's in byte order.
        srcs = [s for m in ('a', 'b', 'c') if (root / m).is_dir()
                for s in sorted(str(f.relative_to(root) if cwd else f) for f in (root / m).rglob('*.scala'))]
        where = ['--sourceroot', '.' if cwd else str(root)]
        bound = BOUND['module']
    else:
        srcs = [str(src)] if src.is_file() else sorted(str(f) for f in src.glob('*.scala'))
        where = []
        bound = BOUND['check']

    def diag(r):
        return (r['out'] + r['err']).replace(str(w), '<work>')
    teq = os.path.abspath(teq)
    jvm_cp = str(SCALA_LIBRARY) + (':' + cp if cp else '')
    r = run([teq, 'compiler', 'build', '--target', 'jvm', '--products', str(w / 'jvm')] + where + ['--classpath', jvm_cp] + srcs,
            bound, cwd=cwd)
    rec['jvm'] = {'rc': r['rc'], 'diag': diag(r), 'classes': masked(tree_shas(w / 'jvm'), w / 'jvm', cwd)}
    r = run([teq, 'compiler', 'check', '--products', str(w / 'products')] + where + (['--classpath', cp] if cp else []) + srcs,
            bound, cwd=cwd)
    rec['products'] = {'rc': r['rc'], 'diag': diag(r), 'classes': masked(tree_shas(w / 'products'), w / 'products', cwd)}
    return rec


def masked(shas, d, generated):
    """A generated case's products: its manifest's bytes without its root and the identity made from it."""
    m = pathlib.Path(d) / 'teq-products.json'
    if not generated or 'teq-products.json' not in shas:
        return shas
    shas = dict(shas)
    del shas['teq-products.json']
    try:
        doc = json.loads(m.read_text())
        doc.pop('root', None)
        doc.pop('identity', None)
        shas['teq-products.json, its root and identity masked'] = hashlib.sha256(
            json.dumps(doc, sort_keys=True).encode()).hexdigest()
    except ValueError:
        shas['teq-products.json, not JSON'] = sha(m)
    return shas


def load_baseline(d):
    """DIR/baseline.json and its sha256, refused unless whole: JSON, the binary named, and one record per unit."""
    path = pathlib.Path(d) / 'baseline.json'
    try:
        data = path.read_bytes()
        doc = json.loads(data)
    except (OSError, ValueError) as e:
        sys.exit(f'no baseline at {path}: {e}')
    if not isinstance(doc, dict) or not doc.get('teq', {}).get('sha256') or not doc.get('records') or \
            doc.get('units') != len(doc['records']):
        sys.exit(f'the baseline at {path} is not whole: its binary, its records or its count of units missing')
    return doc, hashlib.sha256(data).hexdigest()


def version_of(teq):
    r = run([teq, '--version'], 30)
    return {'version': r['out'].strip(), 'sha256': sha(teq)}


def collect(teq, work, units, jobs):
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
        recs = list(pool.map(lambda u: measure(teq, work, u), units))
    return {f"{r['kind']}/{r['name']}": r for r in recs}


# What differs between two runs of one binary and is no output of the compiler's, compared through these and kept raw in
# the records. Each is the text of the one step and field it was seen in (the first two runs of master 5f351df07a,
# 24 Nov, gave 12 differences of these kinds):
# - in a JavaScript build's diagnostics, the class path report a failed build prints from its line `classpath` on: its
#   durations, and the widths of its columns, which the durations set; and what the jar cache held, which the first
#   build of a binary writes and the next reads (`jar caches written`, `from the jar cache`); its counts are compared;
# - in tests/errors' check diagnostics, the analysis JSON's `"ms"` timings (the `--analysis` checks);
# - in tests/cases/windows_paths.scala's interpreter output, the suffix its temporary directory is given
#   (Files.createTempDirectory).
DURATION = re.compile(r'\d[\d,.]* ?ms\b')
JAR_CACHE = re.compile(r',\s*[\d,]+ from the jar cache')
ANALYSIS_MS = re.compile(r'"ms":\{[^{}]*\}')
TEMP_DIR = re.compile(r'(teq win ü)\d+')


def normalized(text, key, part, field):
    if text is None:
        return None
    if (part, field) == ('js', 'diag'):
        lines = text.split('\n')
        if 'classpath' in lines:
            at = lines.index('classpath')
            lines[at:] = [re.sub(' {2,}', ' ', JAR_CACHE.sub('', DURATION.sub('<ms>', l))) for l in lines[at:]
                          if not l.strip().startswith('jar caches written')]
        text = '\n'.join(lines)
    if (part, field) == ('check', 'diag'):
        text = ANALYSIS_MS.sub('"ms":{}', text)
    if (key, part, field) == ('cases/windows_paths', 'interp', 'out'):
        text = TEMP_DIR.sub(r'\1<n>', text)
    return text


def differences(base, head):
    out = []
    for key in sorted(set(base) | set(head)):
        b, h = base.get(key), head.get(key)
        if b is None or h is None:
            out.append((key, 'presence', 'only in the ' + ('head' if b is None else 'baseline')))
            continue
        for part in ('generate', 'check', 'js', 'jvm', 'jvm4', 'products', 'interp'):
            bp, hp = b.get(part), h.get(part)
            if bp is None and hp is None:
                continue
            if bp is None or hp is None:
                out.append((key, part, 'step present on one side only'))
                continue
            if 'timeout' in (bp.get('rc'), hp.get('rc')):
                out.append((key, part, 'timeout'))
                continue
            if bp.get('rc') != hp.get('rc'):
                out.append((key, part, f"exit {bp.get('rc')} against {hp.get('rc')}"))
            if normalized(bp.get('diag'), key, part, 'diag') != normalized(hp.get('diag'), key, part, 'diag'):
                out.append((key, part, 'diagnostics differ'))
            if part == 'js' and bp.get('sha') != hp.get('sha'):
                out.append((key, part, 'bytes differ'))
            if part in ('jvm', 'jvm4', 'products') and bp.get('classes') != hp.get('classes'):
                names = sorted(set(bp['classes']) ^ set(hp['classes'])) or \
                    sorted(k for k in bp['classes'] if bp['classes'][k] != hp['classes'].get(k))
                out.append((key, part, 'file bytes differ: ' + ', '.join(names[:5])))
            if part == 'interp' and any(normalized(bp.get(f), key, part, f) != normalized(hp.get(f), key, part, f)
                                        for f in ('out', 'err')):
                out.append((key, part, 'output differs'))
    return out


# The steps whose output master itself does not settle, each with the row of master's it is booked as: a difference
# there that a rerun clears is said and not counted. Any other step a rerun clears fails the comparison as a
# difference does. A step leaves this list in the commit that mends its row.
MASTER_UNSTABLE = {
    # Booked row: master's determinism defect at four workers, one source and two class files by the typer's schedule
    # (an inline val's declared singleton folded as its constant in one, read from its object in the other).
    ('cases/inline_val_declared_singletons', 'jvm4'):
        "at four workers master writes one of several class files for the package object (5f351df07a: 1 build in 30, 0 in"
        " 10 at one worker; 76ae7baae3: 2 in 10, three outputs): each inline val folded as its constant or read from its"
        " object (C$.MODULE$.b()) by the schedule",
}
RERUNS = 3
# A step of MASTER_UNSTABLE is frozen as the output most of SETTLE builds give, so that the baseline holds the one
# master gives nearly always and a head giving the other is cleared by a rerun.
SETTLE = 10


def settled(samples):
    """The record most of `samples` are, with how many and how many outputs there were."""
    counts = collections.Counter(json.dumps(s, sort_keys=True) for s in samples)
    most, n = counts.most_common(1)[0]
    return dict(json.loads(most), settled=f'{n} of {len(samples)} builds; {len(counts)} outputs')


def settle(teq, work, units, recs):
    by_key = {f'{u[0]}/{u[1]}': u for u in units}
    for key, part in MASTER_UNSTABLE:
        if key in recs and part in recs[key] and key in by_key:
            samples = [recs[key][part]] + [measure(teq, pathlib.Path(work) / f'settle{i}', by_key[key])[part]
                                           for i in range(SETTLE - 1)]
            recs[key][part] = settled(samples)
            print(f"settled {key} {part}: {recs[key][part]['settled']}")


def judge(teq, work, units, base, head):
    """The head's differences from the baseline, each unit with one rerun up to RERUNS times: (key, part, what, verdict)
    where the verdict is 'DIFF' when every run differs, 'UNSTABLE' when a rerun equals the baseline."""
    by_key = {f'{u[0]}/{u[1]}': u for u in units}
    first = differences(base, head)
    reruns = {}
    for key in sorted({k for k, _, _ in first if k in by_key and k in base}):
        reruns[key] = []
        for i in range(RERUNS):
            rec = measure(teq, pathlib.Path(work) / f'rerun{i}', by_key[key])
            again = {(p, w.split(':')[0]) for _, p, w in differences({key: base[key]}, {key: rec})}
            reruns[key].append(again)
            if not again:
                break
    out = []
    for key, part, what in first:
        cleared = [i for i, again in enumerate(reruns.get(key, []), 1) if (part, what.split(':')[0]) not in again]
        out.append((key, part, what, f'UNSTABLE (equal on rerun {cleared[0]})' if cleared else 'DIFF'))
    return out


def plan_outcomes(path):
    """The match plans' checker's log (src/tir/matcher.rs, `check`): its lines, and how many fail. Every outcome but
    `ok` fails (missing, stale, stray, no-owner, or one the log should not hold), and so does a log with no `ok`, or
    none: the checker ran on no reader's entry."""
    p = pathlib.Path(path)
    if not p.is_file():
        return ['plans: no checker log'], 1
    counts, examples = collections.Counter(), collections.defaultdict(list)
    for line in p.read_text(errors='replace').splitlines():
        if line.strip():
            outcome = line.split(' ', 1)[0]
            last = line.rsplit(' ', 1)[-1]
            counts[outcome] += int(last[len('count='):]) if last.startswith('count=') and last[len('count='):].isdigit() else 1
            if outcome != 'ok' and len(examples[outcome]) < 3:
                examples[outcome].append(line[:200])
    lines = [f"plans: {counts['ok']} ok" + ''.join(f', {n} {o}' for o, n in sorted(counts.items()) if o != 'ok')]
    lines += [f'PLAN {o}: {e}' for o, ex in sorted(examples.items()) for e in ex]
    return lines, sum(n for o, n in counts.items() if o != 'ok') + (0 if counts['ok'] else 1)


def verdicts(judged):
    """The judged differences' lines, and how many fail: every DIFF, and every UNSTABLE but master's own."""
    lines, failing = [], 0
    for key, part, what, verdict in judged:
        why = MASTER_UNSTABLE.get((key, part)) if verdict != 'DIFF' else None
        if verdict == 'DIFF':
            lines.append(f'DIFF {key} {part}: {what}')
        else:
            lines.append(f'UNSTABLE {key} {part}: {what}, {verdict.split(" ", 1)[1][1:-1]}; ' +
                         (f"master's own: {why}" if why else "NOT one of master's own"))
        failing += not why
    return lines, failing


PLANT = '''#!/bin/bash
# The self-test's wrapper: the real teq, then one planted difference per chosen test.
real=REAL
"$real" "$@"; rc=$?
out=""; prev=""
for a in "$@"; do case "$prev" in -o|--products) out=$a ;; esac; prev=$a; done
case "$*" in
  *"--target js"*"BYTE_TEST"*|*"BYTE_TEST"*"--target js"*) printf '/* planted */' >> "$out" ;;
esac
case "$*" in
  *"--target jvm"*"DIAG_TEST"*|*"DIAG_TEST"*"--target jvm"*) echo "planted: a diagnostic" >&2 ;;
esac
case "$1 $*" in
  "interp "*"OUT_TEST"*) echo "planted output" ;;
esac
case "$1 $*" in
  "interp "*"ONCE_TEST"*) [ -e "ONCE_MARK" ] || { : > "ONCE_MARK"; echo "planted once"; } ;;
esac
case "$*" in
  *"--target jvm"*"CLASS_TEST"*|*"CLASS_TEST"*"--target jvm"*)
    if [ -d "$out" ]; then f=$(find "$out" -name '*.class' | sort | head -1); [ -n "$f" ] && printf 'x' >> "$f"; fi ;;
esac
case "$*" in
  *"--target jvm"*"MODULE_TEST/"*)
    if [ -d "$out" ]; then f=$(find "$out" -name '*.class' | sort | head -1); [ -n "$f" ] && printf 'x' >> "$f"; fi ;;
esac
case "$*" in
  "compiler check "*"TASTY_TEST"*)
    if [ -d "$out" ]; then f=$(find "$out" -name '*.tasty' | sort | head -1); [ -n "$f" ] && printf 'x' >> "$f"; fi ;;
esac
exit $rc
'''


def self_test(teq, work):
    tests = ['tests/cases/abstract_function.scala', 'tests/cases/abstract_constructor_extractor.scala',
             'tests/cases/abandoned_operand_frames.scala', 'tests/cases/adt.scala']
    module, tasty = 'tests/modules/access', 'tests/tasty/src/bodies.scala'
    only = '^(cases/(' + '|'.join(pathlib.Path(t).stem for t in tests) + ')|modules/access|tasty/src/bodies\\.scala)$'
    units = case_units(only)
    if len(units) != len(tests) + 2:
        print(f'self-test: expected {len(tests) + 2} units, found {len(units)}')
        return 1
    lines = []
    # The comparison's normalizations hide a run's durations and nothing else.
    def same(a, b, *at):
        return normalized(a, *at) == normalized(b, *at)
    report = 'x\n  ^\nclasspath\n  files read   {}   {} ms  6 KB\n'
    js = ('cases/x', 'js', 'diag')
    if not same(report.format(93, '2.04'), report.format(93, '  10.71'), *js):
        lines.append('normalized: two durations of the class path report differ')
    cold = 'x\nclasspath\n  files read  229  8.87 ms  2.7 MB\n  jar caches written  4  1.2 ms  2.7 MB\n'
    warm = 'x\nclasspath\n  files read  229   5.15 ms  2.7 MB, 229 from the jar cache\n'
    if not same(cold, warm, *js) or same(cold, warm.replace('229   5', '230   5'), *js):
        lines.append("normalized: the jar cache's state not hidden, or a count of the report hidden with it")
    if same(report.format(93, '2.04'), report.format(94, '2.04'), *js):
        lines.append('normalized: a count of the class path report hidden')
    if same('a\n  ^\nclasspath\n', 'a\n   ^\nclasspath\n', *js):
        lines.append("normalized: a caret's column before the class path report hidden")
    if same(report.format(93, '2.04'), report.format(93, '10.71'), 'cases/x', 'jvm', 'diag') or \
            same(report.format(93, '2.04'), report.format(93, '10.71'), 'cases/x', 'interp', 'out'):
        lines.append('normalized: the class path report hidden outside a JavaScript build')
    check = ('errors/x', 'check', 'diag')
    if not same('{"ms":{"read":1.5,"total":2}}', '{"ms":{"read":3.25,"total":4}}', *check) or \
            same('{"line":20,"ms":{}}', '{"line":21,"ms":{}}', *check) or \
            same('{"ms":{"read":1.5}}', '{"ms":{"read":3.25}}', 'cases/x', 'js', 'diag'):
        lines.append('normalized: the analysis timings not hidden, a position hidden, or timings hidden elsewhere')
    temp = ('cases/windows_paths', 'interp', 'out')
    if not same('/tmp/teq win ü604/a', '/tmp/teq win ü31/a', *temp) or \
            same('/tmp/teq win ü604/a', '/tmp/teq win ü604/b', *temp) or \
            same('/tmp/teq win ü604/a', '/tmp/teq win ü31/a', 'cases/other', 'interp', 'out'):
        lines.append('normalized: the temporary directory not hidden, its contents hidden, or hidden elsewhere')
    # The comparison's own refusals, over records made for them: a timeout on either side, a program or a step on one
    # side alone, each a failing difference; and a baseline cut, empty or without records refused before any run.
    ok = {'rc': 0, 'diag': '', 'sha': 'a'}
    rec = lambda **parts: dict({'kind': 'cases', 'name': 'x'}, **parts)
    made = [({'cases/x': rec(js=dict(ok, rc='timeout'))}, {'cases/x': rec(js=ok)}, 'timeout'),
            ({'cases/x': rec(js=ok)}, {'cases/x': rec(js=dict(ok, rc='timeout'))}, 'timeout'),
            ({'cases/x': rec(js=dict(ok, rc='timeout'))}, {'cases/x': rec(js=dict(ok, rc='timeout'))}, 'timeout'),
            ({'cases/x': rec(js=ok)}, {}, 'only in the baseline'),
            ({}, {'cases/x': rec(js=ok)}, 'only in the head'),
            ({'cases/x': rec(js=ok, interp={'rc': 0, 'out': '', 'err': ''})}, {'cases/x': rec(js=ok)},
             'step present on one side only'),
            # The run that prints the expected lines and then fails: its exit tells it.
            ({'cases/x': rec(js=ok, interp={'rc': 0, 'out': 'a\n', 'err': ''})},
             {'cases/x': rec(js=ok, interp={'rc': 1, 'out': 'a\n', 'err': ''})}, 'exit 0 against 1')]
    for b, h, what in made:
        got = differences(b, h)
        if [d[2] for d in got] != [what] or verdicts([d + ('DIFF',) for d in got])[1] != 1:
            lines.append(f'differences: {what} not found as one failing difference: {got}')
    good = json.dumps({'teq': version_of(teq), 'units': 1, 'records': {'cases/x': rec(js=ok)}})
    for name, text in (('cut', good[:len(good) // 2]), ('empty', ''), ('no records', json.dumps({'teq': {}, 'units': 0,
                       'records': {}})), ('short', good.replace('"units": 1', '"units": 2'))):
        bad = pathlib.Path(work) / f'baseline-{name.replace(" ", "-")}'
        bad.mkdir(parents=True, exist_ok=True)
        (bad / 'baseline.json').write_text(text)
        try:
            load_baseline(bad)
            lines.append(f'a baseline {name} was read')
        except SystemExit:
            pass
    try:
        load_baseline(pathlib.Path(work) / 'baseline-none')
        lines.append('a missing baseline was read')
    except SystemExit:
        pass
    base = collect(teq, pathlib.Path(work) / 'base', units, 4)
    again = collect(teq, pathlib.Path(work) / 'again', units, 4)
    # The planted differences prove something only over steps that ran: each test builds and runs on every target,
    # and its JVM builds write class files.
    for key, rec in sorted(base.items()):
        steps = ('jvm', 'products') if rec['kind'] in ('modules', 'tasty') else ('js', 'jvm', 'jvm4', 'products', 'interp')
        for part in steps:
            if rec.get(part, {}).get('rc') != 0:
                lines.append(f'{key} {part} did not succeed on the real binary: {str(rec.get(part))[:200]}')
        for part in ('jvm', 'jvm4'):
            if part in steps and not any(n.endswith('.class') for n in rec.get(part, {}).get('classes', {})):
                lines.append(f'{key} {part} wrote no class file')
        if not any(n.endswith('.tasty') for n in rec.get('products', {}).get('classes', {})):
            lines.append(f'{key} products wrote no .tasty')
    same = differences(base, again)
    if same:
        lines.append(f'the same binary twice differs (not deterministic): {same}')
    wrapper = pathlib.Path(os.path.abspath(work)) / 'planted-teq'
    mark = wrapper.with_name('planted-once.mark')
    if mark.exists():
        mark.unlink()
    wrapper.write_text(PLANT.replace('REAL', os.path.abspath(teq)).replace('BYTE_TEST', tests[0])
                       .replace('DIAG_TEST', tests[1]).replace('OUT_TEST', tests[2]).replace('CLASS_TEST', tests[3])
                       .replace('MODULE_TEST', module).replace('TASTY_TEST', tasty)
                       .replace('ONCE_TEST', tests[3]).replace('ONCE_MARK', str(mark)))
    wrapper.chmod(0o755)
    # Through the comparison's own judge: the eight planted in every run stay differences after the reruns; the one
    # planted in the first run alone (adt's interpreter output) is cleared by a rerun, and fails as no step of
    # master's own.
    planted = collect(str(wrapper), pathlib.Path(work) / 'planted', units, 4)
    judged = judge(str(wrapper), pathlib.Path(work) / 'planted', units, base, planted)
    found = {(k, p, d.split(':')[0], v.split()[0]) for k, p, d, v in judged}
    stem = [f'cases/{pathlib.Path(t).stem}' for t in tests]
    want = {(stem[0], 'js', 'bytes differ'),
            (stem[1], 'jvm', 'diagnostics differ'), (stem[1], 'jvm4', 'diagnostics differ'),
            (stem[2], 'interp', 'output differs'),
            (stem[3], 'jvm', 'file bytes differ'), (stem[3], 'jvm4', 'file bytes differ'),
            ('modules/access', 'jvm', 'file bytes differ'), ('tasty/src/bodies.scala', 'products', 'file bytes differ')}
    want = {w + ('DIFF',) for w in want} | {(stem[3], 'interp', 'output differs', 'UNSTABLE')}
    for w in sorted(want - found):
        lines.append(f'planted and NOT found: {w}')
    for f in sorted(found - want):
        lines.append(f'found and not planted: {f}')
    if verdicts(judged)[1] != len(want):
        lines.append(f'{verdicts(judged)[1]} of the {len(want)} planted fail the comparison')
    # A step of master's own that a rerun clears is said and not counted; the same step differing every time is.
    own = next(iter(MASTER_UNSTABLE))
    if verdicts([own + ('x', 'UNSTABLE (equal on rerun 1)')])[1] != 0 or verdicts([own + ('x', 'DIFF')])[1] != 1:
        lines.append("verdicts: a step of master's own counted when a rerun clears it, or not when none does")
    common, rare = {'rc': 0, 'diag': '', 'classes': {'a': '1'}}, {'rc': 0, 'diag': '', 'classes': {'a': '2'}}
    got = settled([rare, common, common])
    if got.get('classes') != common['classes'] or got.get('settled') != '2 of 3 builds; 2 outputs':
        lines.append(f'settled: the output most builds give not taken: {got}')
    # The plans' checker's log: all ok passes; each other outcome fails, and so do an empty log and no log.
    ok_line = 'ok js match Match(TExprId(24)) compiler build x.scala\n'
    for name, text, fails in (('all ok', ok_line * 2, 0), ('counted', 'ok js match count=3\nok jvm match count=1\n', 0), ('missing', ok_line + 'missing jvm match Match(TExprId(9)) a\n', 1),
                              ('stale', ok_line + 'stale interp match Match(TExprId(9)) a\n', 1),
                              ('stray', ok_line + 'stray js try Try(3) a\n', 1),
                              ('no-owner', ok_line + 'no-owner js match Match(TExprId(9)) a\n', 1),
                              ('unknown', ok_line + 'made-after-pass TExprId(9) a\n', 1), ('empty', '', 1), ('absent', None, 1)):
        log = pathlib.Path(work) / f"plans-{name.replace(' ', '-')}.log"
        if text is not None:
            log.write_text(text)
        elif log.exists():
            log.unlink()
        if plan_outcomes(log)[1] != fails:
            lines.append(f'plans: a log {name} gives {plan_outcomes(log)}')
    print('self-test: ' + (f'{len(want)} planted differences found (bytes, diagnostics, output, class bytes, a '
                           "module's class bytes, a program's TASTy, one in the first run alone), none other; the "
                           'normalizations hide what they name, in their step alone; the reruns keep the lasting ones '
                           "and clear the passing one, which fails as no step of master's own; over made records, "
                           'timeouts and one-sided programs and steps fail, cut, empty and short baselines are '
                           'refused, the most frequent output is the one settled, and the plans\' checker fails on all but ok and on no hit; every step ran; the same binary '
                           'twice equal' if not lines else 'FAILED'))
    for l in lines:
        print('  ' + l)
    return 0 if not lines else 1


def main():
    if not SCALA_LIBRARY.is_file():
        sys.exit(f'no scala-library at {SCALA_LIBRARY}: set COURSIER_CACHE')
    ap = argparse.ArgumentParser()
    ap.add_argument('verb', choices=['freeze', 'compare', 'self-test'])
    ap.add_argument('--teq', required=True)
    ap.add_argument('--dir', required=True)
    ap.add_argument('--base')
    ap.add_argument('--jobs', type=int, default=8)
    ap.add_argument('--only')
    ap.add_argument('--extra', action='append', default=[],
                    help='a directory of single-file programs measured as tests/cases ones are (repeatable)')
    ap.add_argument('--check-plans', action='store_true',
                    help="compare with the match plans' checker on (TEQ_CHECK_MATCH_PLANS=DIR/plans.log): any outcome but ok, "
                         'or no reader checked at all, fails')
    a = ap.parse_args()
    if not pathlib.Path('tests/cases').is_dir():
        sys.exit('run from the repository root')
    work = pathlib.Path(a.dir)
    work.mkdir(parents=True, exist_ok=True)
    if a.verb == 'self-test':
        sys.exit(self_test(a.teq, work))
    if a.verb == 'freeze' and (work / 'baseline.json').exists():
        sys.exit(f'{work / "baseline.json"} exists: a frozen baseline is never written over')
    if a.verb == 'compare':
        if not a.base:
            sys.exit('compare needs --base')
        base, base_sha = load_baseline(a.base)
        if a.check_plans:
            if (work / 'plans.log').exists():
                sys.exit(f"{work / 'plans.log'} exists: the checker's log is a run's own")
            os.environ['TEQ_CHECK_MATCH_PLANS'] = os.path.abspath(work / 'plans.log')
    units = case_units(a.only, a.extra)
    recs = collect(a.teq, work / 'work', units, a.jobs)
    if a.verb == 'freeze':
        settle(a.teq, work / 'work', units, recs)
    doc = {'teq': version_of(a.teq), 'units': len(units), 'records': recs,
           'not measured': TASTY_NOT_MEASURED, 'master unstable': {f'{k} {p}': w for (k, p), w in MASTER_UNSTABLE.items()}}
    (work / ('baseline.json' if a.verb == 'freeze' else 'head.json')).write_text(json.dumps(doc, indent=1, sort_keys=True))
    print(f"{a.verb}: {len(units)} units, {sum(1 for r in recs.values() if 'nojar' in r)} without their jars; "
          f"teq {doc['teq']['version']}")
    if a.verb == 'compare':
        print(f"baseline: sha256 {base_sha}, {base['teq']['version']} (binary {base['teq']['sha256'][:16]}), "
              f"{base['units']} units")
        judged = judge(a.teq, work / 'work', units, base['records'], recs)
        lines, failing = verdicts(judged)
        for l in lines:
            print(l)
        if a.check_plans:
            plan_lines, plan_failing = plan_outcomes(work / 'plans.log')
            for l in plan_lines:
                print(l)
            failing += plan_failing
        print(f'differences: {failing}' + (f" (and {len(judged) - failing} of master's own unstable steps)" if len(judged) > failing else ''))
        sys.exit(1 if failing else 0)


if __name__ == '__main__':
    main()
