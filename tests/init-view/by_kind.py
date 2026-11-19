#!/usr/bin/env python3
"""by_kind.py: what keeps teq's view of scalac's OWN pickles (scalac-view.dump, written by tests/init-view-scalac.sh
and tests/init-view-corpus.sh) apart from scalac's InitDump, line by line, by kind (run from the repository's root):

- ORACLE: a fact of scalac's typer tree that no pickle carries, so no view of a pickle can show it:
  - a position: `PositionPickler` writes no span equal to its children's envelope (PositionPickler.scala 119-120:
    `x.span.toSynthetic != x.envelope(x.source) || alwaysNeedsPos(x)`), so the unpickled point is the start. Checked
    against `teq tasty --positions` on Tpt, Apply, Assign, Match, UnApply, a package's Ident and a named SELFDEF;
  - a package prefix: `TreePickler` writes `ThisType` of a non-root package as `TERMREFpkg` (TreePickler.scala
    248-251), which reads back as its `TermRef`;
  - a symbol the typer chose before a synthetic override: `Enum.ordinal` is pickled as a plain `SELECT` on the enum
    case's `this`, which resolves to the override `SyntheticMembers` added;
  - a `Return`'s `from`: `Typer.typedReturn` makes it `Ident(TermRef(NoPrefix, owner))`, `TreeUnpickler` (RETURN)
    `Ident(from.termRef)`, so a class member's prefix reads back as the class's `this`;
  - a parameter accessor's place among a template's statements: `TreePickler` writes the parameters apart
    (`decomposeTemplateBody`) and `readTemplate` reads the body as `tparams ++ vparams ++ stats`;
  - a package clause's qualified id: the pickle carries one `TERMREFpkg`, which `readTerm` reads back as `tpd.ref`'s
    `Ident` (its prefix, the enclosing package's `this`, elidable);
- O1: a library body's source lines (the printer's source identity);
  - a library type whose denotation scalac has not computed when its dump reads it (`currentSymbol` in
    `stopBecauseStaticOrLocal`): its package prefix visited, `paths=[scala]`;
- VIEW: a rule of the view not ported yet;
- other hunks: lines of differing tree shapes.

Counted in scalac's lines (a line of the view's alone, in a hunk, is counted apart)."""
import difflib, re, sys, collections
sys.path.insert(0, 'tests/init-view')
from view_census import pairs
import oracle


def under_return(A, i):
    """Whether line `i` of a dump is a `Return`'s child: the nearest line above it indented less."""
    ind = len(A[i]) - len(A[i].lstrip())
    for j in range(i - 1, -1, -1):
        if len(A[j]) - len(A[j].lstrip()) < ind:
            return A[j].lstrip().startswith('Return ')
    return False


def classify_hunk(a, b):
    """A hunk's ORACLE kind by tests/init-view/oracle.py (the one list compare.py excuses by), else `other hunks`."""
    if oracle.package_id(a, b):
        return oracle.PACKAGE_ID
    ga, gb = oracle.accessor_groups(a), oracle.accessor_groups(b)
    if ga is not None and gb is not None and (ga or gb):
        return oracle.ACCESSOR_PLACE
    return 'other hunks'


def classify(x, y, keys):
    if ' s2 ' in x or x.startswith('source s2') or '?:?' in y or (x.startswith('source ') and ' path=library/' in x):
        return "O1: a library body's source lines"
    # A library type whose denotation scalac has not computed when its dump reads it: `stopBecauseStaticOrLocal`
    # reads `tp.currentSymbol`, which forces none, so the traverser does not stop at the static class and visits its
    # package prefix (shown by a probe of InitDump: every `paths=[scala]` line's `TypeRef` has no current symbol, every
    # other line's has one). Scalac's lazy-completion history, which no pickle carries; to the checker, a package
    # evaluated (Hot).
    if keys == ['paths'] and 'paths=[scala]' in x and 'paths=[]' in y:
        return oracle.LAZY_COMPLETION
    if keys == ['POS']:
        return oracle.POSITION
    if keys == ['pre'] and (('this(' in x and 'term(' in y) or ('term(' in x and 'this(' in y and '{package' in x)):
        return oracle.PACKAGE_PREFIX
    if 'scala.reflect.Enum.ordinal' in x:
        return oracle.ENUM_ORDINAL
    return 'VIEW: ' + x.strip().split(' ')[0] + ' ' + ','.join(keys)


cat = collections.Counter(); ex = {}; total = 0; shape = 0; extra = 0
for n, o, v in pairs():
    A = open(o).read().splitlines(); B = open(v).read().splitlines(); total += len(A)
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, A, B, autojunk=False).get_opcodes():
        if tag == 'equal':
            continue
        if tag == 'replace' and i2 - i1 == j2 - j1:
            for x, y in zip(A[i1:i2], B[j1:j2]):
                fx = dict(re.findall(r' (\w+)=(\S+)', x)); fy = dict(re.findall(r' (\w+)=(\S+)', y))
                k = sorted(f for f in set(fx) | set(fy) if fx.get(f) != fy.get(f))
                px = re.search(r'\[[^\]]*\]', x); py = re.search(r'\[[^\]]*\]', y)
                if px and py and px.group(0) != py.group(0):
                    k.append('POS')
                if re.findall(r'\{[^}]*\}', x) != re.findall(r'\{[^}]*\}', y):
                    k.append('flags')
                if k == ['pre'] and 'pre=none' in x and 'pre=this(' in y and under_return(A, A.index(x, i1)):
                    c = oracle.RETURN_FROM
                else:
                    c = classify(x, y, k)
                cat[c] += 1; ex.setdefault(c, n)
        else:
            c = classify_hunk(A[i1:i2], B[j1:j2])
            if c == 'other hunks':
                shape += i2 - i1
            else:
                cat[c] += i2 - i1; ex.setdefault(c, n)
            extra += max(0, (j2 - j1) - (i2 - i1))
d = sum(cat.values()) + shape
print(f'scalac lines {total}; differing {d} ({100 * d / max(total, 1):.1f}%): in place or classified {sum(cat.values())}, other hunks {shape}; lines of the view alone in hunks {extra}')
groups = collections.Counter()
for c, v in cat.items():
    groups[c.split(':')[0]] += v
print('by group:', dict(groups), 'other hunks', shape)
for c, v in cat.most_common():
    print(f'{v:5d} {c}   e.g. {ex[c]}')
