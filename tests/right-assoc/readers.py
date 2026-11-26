#!/usr/bin/env python3
"""tests/right-assoc/readers.py [--write] | --self-test: three checks of the right-associative clause order's code, each
held to a file of tests/right-assoc:
- readers.txt: the census of the functions in src/ that read a right-associative symbol's clause order by its markers
  (the permutation and its kin, `ext_clauses`, the trailing-':' test), each with its disposition (closure.txt's words:
  PORT, DELETE, BOUNDARY, PINNED, or ORDER-FREE for one that reads a marker and no order). It fails on a function that
  reads a marker and is not in the file (a reader added unseen), and on a line of the file whose function reads no
  marker now (gone or renamed). A function is named by its nearest lexical `fn` above the line, as `file<TAB>function`.
- forbidden.txt: what the swap at the definition deletes, named whatever it reads (a function, an enum, or an exact
  text such as a call or a message), each PENDING (present until the landing) or GONE (absent). It fails on a GONE
  entry found and on a PENDING one missing (mark it GONE), independent of the markers.
- transforms.txt: the census of clause-index transformations in src/ (a clause list or order swapped, rotated,
  reversed, an element moved, arithmetic over `ext_clauses`), each read once with its disposition. It fails on one
  that is not in the file and on a line of the file no longer found.
--write prints the marker census's lines for a new readers.txt. The self-test plants bad records first, each of which
must fail: an added reader, a gone reader, a rogue permutation, a retained deleted function, a compensation that reads
no marker, a pending entry gone."""
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

FORBIDDEN_KINDS = ('fn', 'enum', 'text')
STATES = ('PENDING', 'GONE')
TRANSFORM = re.compile(r"(clauses|params|order)[\w\.\[\]\(\)]*\.(swap|rotate_left|rotate_right|reverse)\(|order\.(insert|remove)\("
                       r"|ext_clauses\s+as\s+usize\s*[-+]\s*\d|[-+]\s*(?:info\.|s\.|self\.)?ext_clauses\b|\.swap\(\s*(first|ext_clauses|receiver)")


def forbidden_rows(path):
    rows = []
    for n, line in enumerate(open(path).read().splitlines(), 1):
        if not line or line.startswith('#'):
            continue
        f = line.split('\t')
        if len(f) < 5 or f[0] not in FORBIDDEN_KINDS or f[3] not in STATES:
            raise SystemExit(f'forbidden.txt:{n}: not `kind<TAB>file<TAB>name or text<TAB>state<TAB>why` with a kind of {FORBIDDEN_KINDS} and a state of {STATES}')
        rows.append((f[0], f[1], f[2], f[3]))
    return rows


def present(root, kind, rel, what):
    p = os.path.join(root, rel)
    if not os.path.exists(p):
        return False
    text = open(p, errors='replace').read()
    if kind == 'text':
        return what in text
    return re.search(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?' + ('fn' if kind == 'fn' else 'enum') + r'\s+' + re.escape(what) + r'\b', text, re.M) is not None


def check_forbidden(root, rows):
    bad = []
    for kind, rel, what, state in rows:
        here = present(root, kind, rel, what)
        if state == 'GONE' and here:
            bad.append(f'forbidden and present: {kind} {rel} {what}')
        if state == 'PENDING' and not here:
            bad.append(f'pending but gone (mark it GONE): {kind} {rel} {what}')
    return bad


def transforms_census(root):
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
                if TRANSFORM.search(line) and not line.strip().startswith('//'):
                    found.add((os.path.relpath(p, root), cur, ' '.join(line.split())))
    return found


def transforms_ledger(path):
    rows = set()
    for n, line in enumerate(open(path).read().splitlines(), 1):
        if not line or line.startswith('#'):
            continue
        f = line.split('\t')
        if len(f) < 4 or f[3] not in KINDS:
            raise SystemExit(f'transforms.txt:{n}: not `file<TAB>function<TAB>line<TAB>kind` with a kind of {KINDS}')
        rows.add((f[0], f[1], f[2]))
    return rows


def check_transforms(found, rows):
    bad = [f'unlisted transformation: {p}\t{fn}\t{l}' for p, fn, l in sorted(found - rows)]
    bad += [f'listed but not found: {p}\t{fn}\t{l}' for p, fn, l in sorted(rows - found)]
    return bad


def planted(files):
    """A tree of the given files (path: text) in a fresh directory, for the self-test."""
    import tempfile
    d = tempfile.mkdtemp(prefix='readers-planted-')
    for rel, text in files.items():
        os.makedirs(os.path.dirname(os.path.join(d, rel)), exist_ok=True)
        open(os.path.join(d, rel), 'w').write(text)
    return d


def self_test_planted():
    import shutil
    n = 0
    # A rogue permutation, in no list: the transformation census must name it.
    d = planted({'src/a.rs': 'fn f(sig: &mut S) {\n    sig.clauses.swap(0, 1);\n}\n'})
    try:
        assert check_transforms(transforms_census(d), set()) != [], 'a rogue permutation passed'
        n += 1
        # A deleted function retained: GONE and present.
        open(os.path.join(d, 'src/b.rs'), 'w').write('pub(super) fn right_assoc_written_lists(&mut self) {}\n')
        assert check_forbidden(d, [('fn', 'src/b.rs', 'right_assoc_written_lists', 'GONE')]) != [], 'a retained deleted function passed'
        n += 1
        # A compensation that reads no marker, retained: found by its name alone.
        open(os.path.join(d, 'src/c.rs'), 'w').write('    fn reordered_operands(&mut self, items: &[u32]) -> Vec<u32> { items.to_vec() }\n')
        assert not MARKERS.search(open(os.path.join(d, 'src/c.rs')).read()), 'the planted compensation reads a marker'
        assert check_forbidden(d, [('fn', 'src/c.rs', 'reordered_operands', 'GONE')]) != [], 'a marker-free compensation passed'
        n += 1
        # A pending entry gone: the file must say GONE.
        assert check_forbidden(d, [('text', 'src/c.rs', 'no such call(', 'PENDING')]) != [], 'a pending entry gone passed'
        n += 1
        # And a clean planted tree passes both.
        assert check_forbidden(d, [('fn', 'src/c.rs', 'reordered_operands', 'PENDING')]) == []
        assert check_transforms(transforms_census(d), transforms_census(d)) == []
    finally:
        shutil.rmtree(d, ignore_errors=True)
    return n


if __name__ == '__main__':
    if sys.argv[1:] == ['--self-test']:
        print(f'readers: self-test: {self_test()} of 3 planted censuses and {self_test_planted()} of 4 planted trees judged as planted')
        sys.exit(0)
    found = census(REPO)
    if sys.argv[1:] == ['--write']:
        for p, fn in sorted(found):
            print(f'{p}\t{fn}')
        sys.exit(0)
    print(f'readers: self-test: {self_test()} of 3 planted censuses and {self_test_planted()} of 4 planted trees judged as planted')
    bad = check(found, ledger(os.path.join(HERE, 'readers.txt')))
    for b in bad:
        print('readers:', b)
    print(f'readers: {len(found)} functions read the order\'s markers' + (', each in tests/right-assoc/readers.txt' if not bad else f', {len(bad)} not as listed'))
    rows = forbidden_rows(os.path.join(HERE, 'forbidden.txt'))
    fbad = check_forbidden(REPO, rows)
    for b in fbad:
        print('readers:', b)
    pending = sum(1 for r in rows if r[3] == 'PENDING')
    print(f'readers: forbidden.txt: {len(rows)} entries, {pending} pending and {len(rows) - pending} gone' + (', each as it says' if not fbad else f', {len(fbad)} not as it says'))
    tfound = transforms_census(REPO)
    tbad = check_transforms(tfound, transforms_ledger(os.path.join(HERE, 'transforms.txt')))
    for b in tbad:
        print('readers:', b)
    print(f'readers: {len(tfound)} clause-index transformations' + (', each in tests/right-assoc/transforms.txt' if not tbad else f', {len(tbad)} not as listed'))
    sys.exit(1 if bad or fbad or tbad else 0)
