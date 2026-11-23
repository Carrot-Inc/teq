#!/usr/bin/env python3
"""lir_gaps.py summary <out> | --self-test

The count of methods each reason of `lir::Unlowered` holds back, over the statistics of a corpus's JVM builds that
tests/support/lir-gaps.sh keeps: <out>/<program>.gaps holds a program's lines
`LIRGAPS<TAB><class>.<name><desc><TAB><reasons>` (the reasons comma-separated in the enum's order, `-` for none) and
its build's `exit <code>`, or `notrun <program> <why>`. Prints the corpus's line, then one line a reason in the enum's
order (read from src/lir.rs): `<reason><TAB><methods><TAB><programs>`, a reason no method holds included. Exits 1
on a record it cannot read whole: a built program with no line, a line of another shape, a reason the enum lacks or
out of its order, a method read twice; --self-test plants each and a control of known counts.
"""
import os
import re
import sys

LIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'src', 'lir.rs')


def reasons(lir_text):
    m = re.search(r'^pub enum Unlowered \{\n(.*?)^\}', lir_text, re.M | re.S)
    return re.findall(r'^    ([A-Z][A-Za-z]*),$', m.group(1), re.M)


def read(out, order):
    rank = {r: i for i, r in enumerate(order)}
    bad, programs = [], {}
    for f in sorted(os.listdir(out)):
        if not f.endswith('.gaps'):
            continue
        name = f[:-len('.gaps')]
        lines = open(os.path.join(out, f)).read().splitlines()
        if lines and lines[0].startswith('notrun '):
            programs[name] = ('notrun', [])
            continue
        if not lines or not re.fullmatch(r'exit -?\d+', lines[-1]):
            bad.append(f'{name}: no exit line')
            continue
        code = int(lines[-1].split()[1])
        methods, seen = [], set()
        for line in lines[:-1]:
            fields = line.split('\t')
            if len(fields) != 3 or fields[0] != 'LIRGAPS' or not fields[1]:
                bad.append(f'{name}: a line of another shape: {line!r}')
                continue
            why = [] if fields[2] == '-' else fields[2].split(',')
            unknown = [w for w in why if w not in rank]
            if unknown:
                bad.append(f'{name}: a reason the enum lacks: {",".join(unknown)}')
                continue
            if [rank[w] for w in why] != sorted({rank[w] for w in why}):
                bad.append(f'{name}: reasons out of the enum\'s order or twice: {fields[2]}')
                continue
            if fields[1] in seen:
                bad.append(f'{name}: the method {fields[1]} read twice')
                continue
            seen.add(fields[1])
            methods.append((fields[1], why))
        if code == 0 and not methods:
            bad.append(f'{name}: built with no statistics line')
        programs[name] = ('built' if code == 0 else 'failed', methods)
    return bad, programs


def summary(out, order):
    bad, programs = read(out, order)
    built = {n: m for n, (k, m) in programs.items() if k == 'built'}
    failed = sum(1 for k, _ in programs.values() if k == 'failed')
    notrun = sum(1 for k, _ in programs.values() if k == 'notrun')
    total = sum(len(m) for m in built.values())
    held = sum(1 for m in built.values() for _, why in m if why)
    lines = [f'lir-gaps: {len(built)} programs built ({failed} failed, {notrun} not run), {total} methods, {held} of them held back by a reason']
    for r in order:
        methods = sum(1 for m in built.values() for _, why in m if r in why)
        progs = sum(1 for m in built.values() if any(r in why for _, why in m))
        lines.append(f'{r}\t{methods}\t{progs}')
    return bad, lines


def self_test():
    import shutil
    import tempfile
    order = ['Alpha', 'Beta', 'Gamma']
    control = {
        'one': 'LIRGAPS\tA.f()V\tAlpha,Gamma\nLIRGAPS\tA.g()V\t-\nexit 0\n',
        'two': 'LIRGAPS\tB.f()V\tAlpha\nexit 0\n',
        'broken': 'LIRGAPS\tC.f()V\tBeta\nexit 1\n',
        'skipped': 'notrun skipped jars missing: x\n',
    }
    plants = {
        'no line': 'exit 0\n',
        'no exit': 'LIRGAPS\tA.f()V\t-\n',
        'short line': 'LIRGAPS\tA.f()V\nexit 0\n',
        'unknown reason': 'LIRGAPS\tA.f()V\tDelta\nexit 0\n',
        'out of order': 'LIRGAPS\tA.f()V\tGamma,Alpha\nexit 0\n',
        'twice': 'LIRGAPS\tA.f()V\tAlpha,Alpha\nexit 0\n',
        'method twice': 'LIRGAPS\tA.f()V\t-\nLIRGAPS\tA.f()V\tBeta\nexit 0\n',
    }
    base = tempfile.mkdtemp(prefix='lir-gaps-self-test-')
    caught = 0
    try:
        for i, (what, text) in enumerate(plants.items()):
            d = os.path.join(base, f'plant{i}')
            os.makedirs(d)
            for n, t in control.items():
                open(os.path.join(d, n + '.gaps'), 'w').write(t)
            open(os.path.join(d, 'planted.gaps'), 'w').write(text)
            bad, _ = summary(d, order)
            if bad:
                caught += 1
            else:
                print(f'lir-gaps self-test: the plant "{what}" was read as whole')
        d = os.path.join(base, 'control')
        os.makedirs(d)
        for n, t in control.items():
            open(os.path.join(d, n + '.gaps'), 'w').write(t)
        bad, lines = summary(d, order)
        want = ['lir-gaps: 2 programs built (1 failed, 1 not run), 3 methods, 2 of them held back by a reason', 'Alpha\t2\t2', 'Beta\t0\t0', 'Gamma\t1\t1']
        control_ok = not bad and lines == want
        if not control_ok:
            print(f'lir-gaps self-test: the control read {bad or lines}, not {want}')
        real = reasons(open(LIR).read())
        enum_ok = len(real) > 0 and len(set(real)) == len(real)
        if not enum_ok:
            print('lir-gaps self-test: no reason read from src/lir.rs')
    finally:
        shutil.rmtree(base)
    print(f'lir-gaps self-test: {caught} of {len(plants)} plants caught, the control {"passed" if control_ok else "FAILED"}, {len(real)} reasons in src/lir.rs')
    return caught == len(plants) and control_ok and enum_ok


def main(argv):
    if argv == ['--self-test']:
        return 0 if self_test() else 1
    if len(argv) == 2 and argv[0] == 'summary':
        bad, lines = summary(argv[1], reasons(open(LIR).read()))
        for b in bad:
            print(f'BAD {b}')
        print('\n'.join(lines))
        return 1 if bad else 0
    print(__doc__.strip().split('\n')[0], file=sys.stderr)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
