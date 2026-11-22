#!/usr/bin/env python3
"""names.py <answer.json> <scalac.txt> | --self-test: the used names of lvn.Use in teq's analysis answer
(`--analysis-version 3`) against scalac's own record of the same client (scalac.txt, record.sh: -Ydump-sbt-inc's
Used Names of module class Use$). Each of teq's names is compared in scalac's encoding (NameTransformer: `+` is
`$plus`, `;` is `$u003B`), and the two sets must be equal but for one mapping, PROXY below.

PROXY: an inline parameter's binding in an expansion is teq's used name under the parameter's own name, `idx`, where
scalac's record holds the expansion's local proxy, `idx$proxy1` to `idx$proxy4` (one an expansion of `walk`). zinc
invalidates a client by the names whose hashes changed in an upstream API, and no API holds a local proxy: scalac's
names invalidate nothing, and teq's `idx` at most compiles the client again where a member named `idx` changes. The
mapping must name a difference: where teq's names hold the proxies, or scalac's hold none, it fails, to be taken out.

Then a control on the answer itself: the answer with the default getter's name taken out, with the operator's taken
out, and with the three names the library's body adds taken out (what a build that reads the default as the value it
read records) must each be refused. --self-test runs the comparison on planted answers alone."""
import json, re, sys

PROXY = ('idx', re.compile(r'^idx\$proxy[0-9]+$'))
BODY = ('walk$default$1', '$plus', 'idx')
CLASS = 'lvn.Use'
CODES = {'+': '$plus', '-': '$minus', '*': '$times', '/': '$div', '<': '$less', '>': '$greater', '=': '$eq',
         '!': '$bang', '&': '$amp', '|': '$bar', '^': '$up', '%': '$percent', '~': '$tilde', '?': '$qmark',
         ':': '$colon', '#': '$hash', '@': '$at', '\\': '$bslash'}


def encode(name):
    """scalac's encoding of a used name (NameTransformer.encode): an operator character by its code, `;` and any
    other character outside a Java identifier by `$u` and its four hex digits."""
    out = []
    for ch in name:
        if ch in CODES:
            out.append(CODES[ch])
        elif ch.isalnum() or ch in '_$':
            out.append(ch)
        else:
            out.append('$u%04X' % ord(ch))
    return ''.join(out)


def teq_names(answer):
    """The used names teq's answer records for CLASS (its files' `deps`), encoded."""
    found = []

    def walk(x):
        if isinstance(x, dict):
            deps = x.get('deps')
            if isinstance(deps, dict) and CLASS in deps:
                found.append(deps[CLASS].get('names', []))
            for v in x.values():
                walk(v)
        elif isinstance(x, list):
            for v in x:
                walk(v)
    walk(answer)
    if len(found) != 1:
        raise SystemExit(f'the answer records {CLASS} {len(found)} times, not once')
    return {encode(n) for n in found[0]}


def compare(teq, scalac):
    """The differences, one line each; none where the two sets agree under PROXY."""
    bad = []
    own, pattern = PROXY
    proxies = {n for n in scalac if pattern.match(n)}
    if any(pattern.match(n) for n in teq) or not proxies:
        bad.append(f'the PROXY mapping names no difference (teq records {sorted(n for n in teq if pattern.match(n))}, scalac {sorted(proxies)}): take it out')
    if own not in teq:
        bad.append(f"only scalac: {', '.join(sorted(proxies))} (teq's binding {own!r} is not recorded)")
    t = teq - {own}
    s = scalac - proxies
    for n in sorted(s - t):
        bad.append(f'only scalac: {n}')
    for n in sorted(t - s):
        bad.append(f'only teq: {n}')
    return bad


def control(teq, scalac):
    """The answer with the getter's, the operator's, and all three of the body's names taken out: each refused."""
    held = []
    for drop in (('walk$default$1',), ('$plus',), BODY):
        if not compare(teq - set(drop), scalac):
            held.append(drop)
    return held


def self_test():
    scalac = {'$plus', 'Int', 'Walk', 'idx$proxy1', 'idx$proxy2', 'size', 'walk$default$1', 'java$u003Blang$u003B'}
    good = {'$plus', 'Int', 'Walk', 'idx', 'size', 'walk$default$1', 'java$u003Blang$u003B'}
    assert compare(good, scalac) == [], ('control', compare(good, scalac))
    assert encode('+') == '$plus' and encode('java;lang;') == 'java$u003Blang$u003B', 'encoding'
    planted = [('the getter missing', good - {'walk$default$1'}), ('the operator missing', good - {'$plus'}),
               ("the body's three missing", good - set(BODY)), ('an extra name', good | {'extra'}),
               ('the proxies recorded', (good - {'idx'}) | {'idx$proxy1', 'idx$proxy2'}),
               ('the binding unrecorded', good - {'idx'})]
    for name, t in planted:
        assert compare(t, scalac), ('planted passed', name)
    assert compare(good, scalac - {'idx$proxy1', 'idx$proxy2'}), 'a mapping naming no difference passed'
    assert control(good, scalac) == [], 'the control held a planted answer'
    print(f'self-test names.py: {len(planted) + 1} planted answers refused, the control held')


if __name__ == '__main__':
    if sys.argv[1:] == ['--self-test']:
        self_test()
        sys.exit(0)
    teq = teq_names(json.load(open(sys.argv[1])))
    scalac = {l.strip() for l in open(sys.argv[2]) if l.strip()}
    bad = compare(teq, scalac)
    for b in bad:
        print(b)
    held = control(teq, scalac)
    for d in held:
        print('the control passed with', ', '.join(d), 'taken out of the answer')
    sys.exit(1 if bad or held else 0)
