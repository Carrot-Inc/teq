#!/usr/bin/env python3
"""run.py <teq> <scala-library jar> [--all | cell-prefix ...]: tests/constants/run.sh's judge. By default the cells of the
twelve pairs (a declared singleton's inline val in an object, a companion and through an export) and one control of
every other kind, in an object; --all takes every cell of gen.py. For each cell and each target (jvm, js), one build at one worker and one at four workers under TEQ_FORCE_CONST=unpublished:<the cell's
constant> (TEQ_TYPER_THRESHOLD=0, so that the small program takes the four). A pair is one form where the two outputs
are the same bytes (the JVM's class files, each in order; JavaScript's file), or the same refusal at both counts, and the
hook FIRED where its log line names two different threads and `met`. A pair is a fault where it is not one form and
tests/pending/constants.txt does not list it, where it is listed and one form (the list is to be updated), or where it is one form, of the race's cells,
and the hook did not fire. The race's cells (RACE: a declared singleton's inline val outside the top level) are the ones
the race reaches; one form there is a proof only where the hook forced the schedule. Two forms need no firing: the
second form is the defect, met by the hook or by the race itself, whose publication may happen under the loader's
lock, where the hook never holds. Any other cell is a control: an inferred val's reader completes the signature by
typing the initialiser itself, so no other thread holds it and the hook has nothing to force. Self-tests on
planted logs and outputs first."""
import hashlib, os, re, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
DEFAULT = ['inl-single-lib-object', 'inl-single-lib-companion', 'inl-single-lib-export', 'inl-single-loc-object',
           'inl-single-loc-companion', 'inl-single-loc-export', 'inl-lit-object', 'inl-infer-object', 'fin-infer-object',
           'fin-lit-object', 'fin-single-object', 'fin-single-lib-object', 'str-inl-object']
NAMES = {'inl-single-lib': 'b', 'inl-single-loc': 'c', 'inl-lit': 'd', 'inl-infer': 'e', 'fin-infer': 'f', 'fin-lit': 'g',
         'fin-single': 'h', 'fin-single-lib': 'k', 'str-inl': 's'}


def fired(log_text):
    for line in log_text.splitlines():
        f = line.split('\t')
        if len(f) >= 5 and f[0] == 'unpublished' and f[4] == 'met' and f[3] not in ('none', f[2]):
            return True
    return False


def digest(path):
    h = hashlib.sha256()
    if os.path.isdir(path):
        for d, _, fs in sorted(os.walk(path)):
            for f in sorted(fs):
                p = os.path.join(d, f)
                h.update(os.path.relpath(p, path).encode())
                h.update(open(p, 'rb').read())
    else:
        h.update(open(path, 'rb').read())
    return h.hexdigest()[:16]


def race(cell):
    """A cell the race reaches: a declared singleton's inline val, outside the top level."""
    return cell.startswith('inl-single-') and not cell.endswith('-toplevel')


def judge(pairs, pending):
    """pairs: {(cell, target): (one_form, fired)}. The faults, as lines."""
    faults = []
    for key, (one, hook) in sorted(pairs.items()):
        name = f'{key[0]} {key[1]}'
        if one and not hook and race(key[0]):
            faults.append(f'{name}: one form, the hook did not fire')
        if not one and name not in pending:
            faults.append(f'{name}: two forms, not listed')
        if one and name in pending:
            faults.append(f'{name}: one form, still listed')
    return faults


def selftest():
    assert not fired('unpublished\tb\tThreadId(2)\tThreadId(2)\tmet\n'), 'one thread passed'
    assert not fired('unpublished\tb\tThreadId(2)\tnone\tnot met\n'), 'not met passed'
    assert fired('unpublished\tb\tThreadId(2)\tThreadId(5)\tmet\n')
    assert judge({('a', 'jvm'): (False, True)}, set()) == ['a jvm: two forms, not listed']
    assert judge({('a', 'jvm'): (True, True)}, {'a jvm'}) == ['a jvm: one form, still listed']
    assert judge({('inl-single-lib-object', 'jvm'): (True, False)}, set()) == ['inl-single-lib-object jvm: one form, the hook did not fire']
    assert judge({('inl-single-lib-object', 'jvm'): (False, False)}, {'inl-single-lib-object jvm'}) == [], 'two forms faulted for its hook'
    assert judge({('a', 'jvm'): (True, False)}, set()) == [], 'a control faulted for its hook'

    assert judge({('a', 'jvm'): (False, True)}, {'a jvm'}) == []
    print('constants: self-test: 5 planted bad records refused, 4 controls passed')


def build(teq, sl, src, w, target, out, extra):
    env = dict(os.environ, TEQ_THREADS=str(w), TEQ_TYPER_THRESHOLD='0', TEQ_NO_CACHE='1', **extra)
    files = [os.path.join(src, 'Defs.scala'), os.path.join(src, 'Use.scala')]
    cmd = [teq, 'compiler', 'build'] + files + (['--target', 'jvm', '--classpath', sl, '-o', out] if target == 'jvm' else ['-o', out])
    r = subprocess.run(cmd, capture_output=True, text=True, timeout=120, env=env)
    return digest(out) if r.returncode == 0 else 'refused:' + hashlib.sha256((r.stdout + r.stderr).encode()).hexdigest()[:16]


def main():
    selftest()
    teq, sl = sys.argv[1], sys.argv[2]
    every = sys.argv[3:] == ['--all']
    prefixes = [] if every else sys.argv[3:]
    pending_path = os.path.join(ROOT, 'tests', 'pending', 'constants.txt')
    pending = {l.strip() for l in open(pending_path) if l.strip() and not l.startswith('#')} if os.path.exists(pending_path) else set()
    pairs = {}
    with tempfile.TemporaryDirectory() as tmp:
        subprocess.run([sys.executable, os.path.join(HERE, 'gen.py'), os.path.join(tmp, 'cells')], check=True, capture_output=True)
        for cell in sorted(os.listdir(os.path.join(tmp, 'cells', 'cells'))):
            if prefixes and not any(cell.startswith(p) for p in prefixes):
                continue
            if not prefixes and not every and cell not in DEFAULT:
                continue
            name = NAMES[re.sub(r'-(object|toplevel|companion|export)$', '', cell)]
            src = os.path.join(tmp, 'cells', 'cells', cell)
            for target in ('jvm', 'js'):
                ext = '' if target == 'jvm' else '.js'
                one = build(teq, sl, src, 1, target, os.path.join(tmp, f'{cell}-{target}-one{ext}'), {})
                log = os.path.join(tmp, f'{cell}-{target}.log')
                forced = build(teq, sl, src, 4, target, os.path.join(tmp, f'{cell}-{target}-forced{ext}'), {'TEQ_FORCE_CONST': f'unpublished:{name}', 'TEQ_FORCE_CONST_LOG': log})
                hook = fired(open(log).read()) if os.path.exists(log) else False
                pairs[(cell, target)] = (one == forced, hook)
    faults = judge(pairs, pending)
    two = sum(1 for one, _ in pairs.values() if not one)
    for f in faults:
        print('constants: ' + f)
    print(f'constants: {len(pairs)} pairs, {len(pairs) - two} one form, {two} two forms ({len(pending)} listed), {len(faults)} faults')
    sys.exit(1 if faults else 0)


if __name__ == '__main__':
    main()
