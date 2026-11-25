#!/usr/bin/env python3
"""tests/right-assoc/readers.py [--write] | --self-test: the census of the functions in src/ that read a
right-associative symbol's clause order by its markers (the permutation and its kin, `ext_clauses`, the trailing-':'
test), held to tests/right-assoc/readers.txt, where each such function has its disposition (closure.txt's words:
PORT, DELETE, BOUNDARY, PINNED, or ORDER-FREE for one that reads a marker and no order). The check fails on a function
that reads a marker and is not in the file (a reader added unseen), and on a line of the file whose function reads
no marker now (gone or renamed). --write prints the census's lines for a new file. A function is named by its nearest
lexical `fn` above the line, as `file<TAB>function`."""
import os, re, sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
MARKERS = re.compile(r"right_assoc_order|ext_clauses|is_right_assoc_extension|declared_clause_order|jvm_param_order"
                     r"|right_assoc_declared_order|signature_param_index|right_assoc_written_lists|ends_with\(':'\)")
FN = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:unsafe\s+)?fn\s+([a-z_][a-z0-9_]*)')
KINDS = ('PORT', 'DELETE', 'BOUNDARY', 'PINNED', 'ORDER-FREE')


def census(root):
    found = set()
    for dp, _, fs in os.walk(os.path.join(root, 'src')):
        for f in sorted(fs):
            if not f.endswith('.rs'):
                continue
            p = os.path.join(dp, f)
            cur = '-'
            for line in open(p, errors='replace'):
                m = FN.match(line)
                if m:
                    cur = m.group(1)
                if MARKERS.search(line) and not line.strip().startswith('//'):
                    found.add((os.path.relpath(p, root), cur))
    return found


def ledger(path):
    rows = {}
    for n, line in enumerate(open(path).read().splitlines(), 1):
        if not line or line.startswith('#'):
            continue
        f = line.split('\t')
        if len(f) < 4 or f[2] not in KINDS:
            raise SystemExit(f'readers.txt:{n}: not `file<TAB>function<TAB>kind<TAB>why` with a kind of {KINDS}')
        rows[(f[0], f[1])] = f[2]
    return rows


def check(found, rows):
    bad = [f'unlisted reader: {p}\t{fn}' for p, fn in sorted(found - set(rows))]
    bad += [f'listed but reads no marker: {p}\t{fn}' for p, fn in sorted(set(rows) - found)]
    return bad


def self_test():
    rows = {('src/a.rs', 'f'): 'PORT'}
    assert check({('src/a.rs', 'f')}, rows) == []
    assert check({('src/a.rs', 'f'), ('src/b.rs', 'g')}, rows) == ['unlisted reader: src/b.rs\tg'], 'an added reader passed'
    assert check(set(), rows) == ['listed but reads no marker: src/a.rs\tf'], 'a gone reader passed'
    return 3


if __name__ == '__main__':
    if sys.argv[1:] == ['--self-test']:
        print(f'readers: self-test: {self_test()} of 3 planted censuses judged as planted')
        sys.exit(0)
    found = census(REPO)
    if sys.argv[1:] == ['--write']:
        for p, fn in sorted(found):
            print(f'{p}\t{fn}')
        sys.exit(0)
    print(f'readers: self-test: {self_test()} of 3 planted censuses judged as planted')
    bad = check(found, ledger(os.path.join(HERE, 'readers.txt')))
    for b in bad:
        print('readers:', b)
    print(f'readers: {len(found)} functions read the order\'s markers' + (', each in tests/right-assoc/readers.txt' if not bad else f', {len(bad)} not as listed'))
    sys.exit(1 if bad else 0)
