#!/usr/bin/env python3
"""tests/java-reader/progress.py <teq> <out> [--sample N] [--jobs J] [--extra DIR]: no Java source stops the reader, and
none it builds is one scalac refuses. The inputs are corpus.py's: 300 of dotty's .java tests at the tag, cut and
broken with a fixed seed, and the sources of tests/java-reader/broken. Each input is the file beside a Main.scala,
checked by `teq compiler check --target jvm` at one worker and at eight. Every check is to end within two seconds,
building, or refused with an error at a place (`<file>:<line>:<col>: error`). A timeout, a crash, or a refusal
without a place fails the run. An input that is not UTF-8 is refused naming its file, exit 2 and the line `cannot
read <file>: stream did not contain valid UTF-8` exactly, counted apart; any other end of it fails the run, as that
line does for an input that is UTF-8.
An input the head builds is judged by tests/java-reader/judged.tsv (judge.py writes it: master's verdict with the
files named, scalac 3.8.4's and its errors, a listed class by those errors, and the class's reached twin): where
master refuses it and scalac accepts it, it is a turned row, counted; where scalac refuses it, the run fails unless
the row is listed, each class counted apart. "annotation names" (the reader resolves no annotation's name, reached or
not): the reached twin is to build. "unreached class's names" (the reader resolves the names of a class a Scala source
reaches): the reached twin is to be refused at scalac's place, its name not found or not a member. "scalac crashes":
javac judges, and the head is to build where javac accepts. The twin is the input beside a Scala file in its package
that reaches the class. A listed row the head does not build fails the run (the list is stale), and an input the
file does not hold fails it too (judge.py writes it again).
--sample N checks every N-th input (the gate's line); --extra DIR adds the .java files of DIR, each an input as it is
and judged by nothing; --no-judge checks every input for its end alone (a corpus other than the tag's, which
judged.tsv does not hold). -> <out>/progress.tsv, and a line of counts."""
import concurrent.futures, glob, os, re, shutil, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import corpus

args = sys.argv[1:]
TEQ, OUT = args[0], args[1]
def opt(name, default):
    return args[args.index(name) + 1] if name in args else default
SAMPLE, JOBS, EXTRA = int(opt('--sample', '1')), int(opt('--jobs', str(os.cpu_count() or 4))), opt('--extra', None)
JUDGE = '--no-judge' not in args
TAG = corpus.tag()
SL = subprocess.run(['bash', '-c', '. tests/support/jars.sh; jar_of scala-library'], capture_output=True, text=True, cwd=corpus.ROOT).stdout.strip()
files, inputs, broken = corpus.generated(TAG)
if not files:
    sys.exit(f'progress: no .java file under {TAG}/tests (set SCALA3_TAG)')
if EXTRA:
    for f in sorted(glob.glob(os.path.join(EXTRA, '**', '*.java'), recursive=True)):
        inputs.append((f'extra:{os.path.relpath(f, EXTRA)}', os.path.basename(f), corpus.read(f)))
inputs = inputs[::SAMPLE]
judged = {}
for l in open(os.path.join(corpus.HERE, 'judged.tsv'), encoding='utf-8'):
    if not l.startswith('#'):
        k, master, scalac, errors, listed, twin, matched = l.rstrip('\n').split('\t')
        judged[k] = (master, scalac, listed, twin)

LOCATED = re.compile(r'^\S+\.(java|scala):\d+:\d+: error', re.M)
NOT_UTF8 = 'not UTF-8, refused naming the file'
TURNED = 'turned: master refuses, scalac accepts'
LISTED = {'annotation names': 'listed: annotation names', "unreached class's names": "listed: unreached class's names",
          'scalac crashes; javac accepts': 'listed: scalac crashes, javac accepts'}

def twin_answer(key, name, text, twin, listed):
    """The reached twin of a listed row: '' where the head answers it as the class says, else a failure's words."""
    pkg, cls, arity, place = twin.split('|')
    w = corpus.workdir(OUT, 'twin:' + key)
    os.makedirs(w, exist_ok=True)
    if not cls:
        # a file of imports alone: a class after them, for a Scala source to reach
        cls, text = 'TeqReached', text + '\nclass TeqReached {}\n'
    corpus.write(os.path.join(w, name), text)
    corpus.write(os.path.join(w, 'Main.scala'), corpus.MAIN)
    args_ = '[' + ', '.join(['?'] * int(arity)) + ']' if int(arity) else ''
    corpus.write(os.path.join(w, 'TeqReach.scala'), (f'package {pkg}\n' if pkg else '') +
                 f'object TeqReach {{ def reach(x: {cls}{args_}): Int = x.hashCode }}\n')
    try:
        r = subprocess.run([TEQ, 'compiler', 'check', w, '--target', 'jvm', '--classpath', SL, '--threads', '1'],
                           capture_output=True, text=True, timeout=10, errors='replace')
    except subprocess.TimeoutExpired:
        return 'the reached twin timed out'
    finally:
        shutil.rmtree(w, ignore_errors=True)
    out = r.stdout + r.stderr
    if listed == 'annotation names':
        return '' if r.returncode == 0 else 'the reached twin is refused: ' + ' '.join(out.split())[:120]
    at = re.compile(rf'^\S*{re.escape(name)}:{re.escape(place)}: error: (Not found: |.* is not a member of )', re.M)
    return '' if r.returncode == 1 and at.search(out) else f'the reached twin is not refused at {place}: ' + ' '.join(out.split())[:120]
def check(item):
    key, name, text = item
    w = corpus.workdir(OUT, key)
    os.makedirs(w, exist_ok=True)
    path = os.path.join(w, name)
    corpus.write(path, text)
    unread = f'cannot read {path}: stream did not contain valid UTF-8'
    corpus.write(os.path.join(w, 'Main.scala'), corpus.MAIN)
    verdicts = []
    for threads in ('1', '8'):
        try:
            r = subprocess.run([TEQ, 'compiler', 'check', w, '--target', 'jvm', '--classpath', SL, '--threads', threads],
                               capture_output=True, text=True, timeout=2, errors='replace')
            out = r.stdout + r.stderr
            if not corpus.utf8(text):
                if r.returncode == 2 and out.strip() == unread:
                    verdicts.append(NOT_UTF8)
                else:
                    verdicts.append(f'FAIL not UTF-8, exit {r.returncode}: ' + ' '.join(out.split())[:120])
            elif unread in out:
                verdicts.append('FAIL a UTF-8 input read as not UTF-8')
            elif r.returncode == 0:
                verdicts.append('builds')
            elif r.returncode == 1 and LOCATED.search(out):
                verdicts.append('refused at a place')
            else:
                verdicts.append(f'FAIL exit {r.returncode}: ' + ' '.join(out.split())[:120])
        except subprocess.TimeoutExpired:
            verdicts.append('FAIL timeout (2 s)')
    shutil.rmtree(w, ignore_errors=True)
    master, scalac, listed, twin = judged.get(key, (None, None, None, None))
    if not JUDGE:
        pass
    elif 'builds' in verdicts and not key.startswith('extra:'):
        if master is None:
            verdicts.append('FAIL not judged (tests/java-reader/judge.py writes judged.tsv again)')
        elif scalac != 'accepts' and listed in LISTED:
            failed = twin_answer(key, name, text, twin, listed) if twin else ''
            verdicts.append(f'FAIL listed ({listed}) but {failed}' if failed else LISTED[listed])
        elif scalac != 'accepts':
            verdicts.append(f'FAIL built where scalac refuses ({scalac}; {listed or "not listed"})'[:200])
        elif master == 'refuses':
            verdicts.append(TURNED)
    elif listed:
        verdicts.append(f'FAIL listed ({listed}) and not built: the list is stale (judge.py writes it again)')
    return key, verdicts

os.makedirs(OUT, exist_ok=True)
with concurrent.futures.ThreadPoolExecutor(JOBS) as ex:
    results = list(ex.map(check, inputs))
counts = {'builds': 0, 'refused at a place': 0, NOT_UTF8: 0, TURNED: 0, 'FAIL': 0, **{v: 0 for v in LISTED.values()}}
with open(os.path.join(OUT, 'progress.tsv'), 'w') as fh:
    for key, vs in results:
        fh.write(key + '\t' + '\t'.join(vs) + '\n')
        for v in vs:
            counts['FAIL' if v.startswith('FAIL') else v] += 1
fails = [(k, v) for k, vs in results for v in vs if v.startswith('FAIL')]
opposite = sum(1 for k, vs in results if 'refused at a place' in vs and judged.get(k, ('', ''))[1] == 'accepts')
by_scalac = sum(1 for _, v in fails if v.startswith('FAIL built where scalac refuses'))
for k, v in fails[:20]:
    print(f'  {k}: {v}')
print(f'java-reader progress: {len(inputs)} inputs from {len(files)} files and {broken} broken sources (every {SAMPLE}), checked at 1 and 8 workers: '
      f'{counts["builds"]} builds, {counts["refused at a place"]} refused at a place, '
      f'{counts[NOT_UTF8]} not UTF-8 refused naming the file; judged: {counts[TURNED]} turned (master refuses, scalac accepts), '
      f'listed {counts[LISTED["annotation names"]]} annotation names, {counts[LISTED[chr(39).join(["unreached class", "s names"])]]} unreached class\'s names, '
      f'{counts[LISTED["scalac crashes; javac accepts"]]} scalac crashes (javac accepts), {by_scalac} built where scalac refuses; {opposite} refused where scalac accepts; {counts["FAIL"]} failing')
sys.exit(1 if fails else 0)
