#!/usr/bin/env python3
"""Compares `teq classfile` with `javap -s -p` on one class file: the members one prints have to
be the members the other prints, with the same types. javap's Java syntax is rewritten into the
printer's Scala-like syntax and both are reduced to `[static ]name[tparams](types): result`.

usage: javap_compare.py <teq> <class file>
"""
import re
import subprocess
import sys

PRIMS = {'int': 'Int', 'long': 'Long', 'double': 'Double', 'boolean': 'Boolean', 'char': 'Char',
         'byte': 'Byte', 'short': 'Short', 'float': 'Float', 'void': 'Unit'}
CLASH = {'java.lang.Boolean', 'java.lang.Byte', 'java.lang.Short', 'java.lang.Long', 'java.lang.Float',
         'java.lang.Double'}
MODS = {'public', 'private', 'protected', 'static', 'final', 'abstract', 'default', 'synchronized',
        'native', 'strictfp', 'transient', 'volatile'}


def split_top(s, sep=','):
    """Splits at the separators outside brackets; `<:` and `>:` are bounds, not brackets."""
    parts, depth, cur = [], 0, ''
    for i, ch in enumerate(s):
        bound = ch in '<>' and s[i + 1:i + 2] == ':'
        if ch in '<[(' and not bound:
            depth += 1
        elif ch in '>])' and not bound:
            depth -= 1
        if ch == sep and depth == 0:
            parts.append(cur.strip())
            cur = ''
        else:
            cur += ch
    if cur.strip():
        parts.append(cur.strip())
    return parts


def jtype(t):
    t = t.strip()
    if t.endswith('...'):
        return jtype(t[:-3]) + '*'
    if t.endswith('[]'):
        return 'Array[' + jtype(t[:-2]) + ']'
    if t == '?':
        return '?'
    if t.startswith('? extends '):
        return '? <: ' + jtype(t[10:])
    if t.startswith('? super '):
        return '? >: ' + jtype(t[8:])
    if t in PRIMS:
        return PRIMS[t]
    args = ''
    if t.endswith('>'):
        i = t.index('<')
        args = '[' + ', '.join(jtype(a) for a in split_top(t[i + 1:-1])) + ']'
        t = t[:i]
    if t in CLASH:
        return t + args
    return t.rsplit('.', 1)[-1].replace('$', '.') + args


def tparam(p):
    if ' extends ' in p:
        name, bounds = p.split(' extends ', 1)
        return name + ' <: ' + ' & '.join(jtype(b) for b in split_top(bounds, '&'))
    return p.strip()


def tparams_prefix(line):
    """The `<...>` at the start of a javap line as `[...]`, and the rest of the line."""
    if not line.startswith('<'):
        return '', line
    depth = 0
    for i, ch in enumerate(line):
        if ch == '<':
            depth += 1
        elif ch == '>':
            depth -= 1
            if depth == 0:
                break
    return '[' + ', '.join(tparam(p) for p in split_top(line[1:i])) + ']', line[i + 1:].strip()


def javap_member(line):
    line = line.strip().rstrip(';')
    throws = ''
    if ' throws ' in line:
        line, thr = line.split(' throws ', 1)
        throws = ' throws ' + ', '.join(jtype(x) for x in split_top(thr))
    words = line.split(' ')
    static = ''
    while words and words[0] in MODS:
        if words[0] == 'static':
            static = 'static '
        words.pop(0)
    tps, line = tparams_prefix(' '.join(words))
    if '(' in line:
        head, params = line.split('(', 1)
        params = params.rstrip(')')
        ps = ', '.join(jtype(p) for p in split_top(params))
        head = head.strip()
        if ' ' in head:
            ret, name = head.rsplit(' ', 1)
            return f'{static}{name}{tps}({ps}): {jtype(ret)}{throws}'
        return f'{static}this{tps}({ps}){throws}'
    ty, name = line.rsplit(' ', 1)
    return f'{static}{name}: {jtype(ty)}'


def teq_member(line):
    line = re.sub(r' = .*$', '', line.strip())
    line = re.sub(r'^(@[\w.]+ )*', '', line)
    line = re.sub(r'^(private\[[^\]]*\] |private |protected )', '', line)
    static = ''
    words = line.split(' ')
    while words and words[0] in ('static', 'final', '<bridge>', '<synthetic>'):
        if words[0] == 'static':
            static = 'static '
        words.pop(0)
    if not words or words[0] not in ('def', 'val', 'var'):
        return None
    line = ' '.join(words[1:])
    if '(' in line:
        head, rest = line.split('(', 1)
        depth, i = 1, 0
        while depth > 0:
            if rest[i] == '(':
                depth += 1
            elif rest[i] == ')':
                depth -= 1
            i += 1
        params = rest[:i - 1]
        types = ', '.join(p.split(': ', 1)[1] for p in split_top(params))
        return f'{static}{head}({types}){rest[i:]}'
    return static + line


def main():
    teq, path = sys.argv[1], sys.argv[2]
    out = subprocess.run(['javap', '-s', '-p', path], capture_output=True, text=True, timeout=60)
    if out.returncode != 0:
        print('javap failed:', out.stderr)
        return 2
    theirs = []
    for line in out.stdout.splitlines():
        s = line.strip()
        if not s or s.startswith('descriptor:') or s.startswith('Compiled from') or s.endswith('{') or s == '}' \
                or s.startswith('static {}'):
            continue
        theirs.append(javap_member(s))
    out = subprocess.run([teq, 'classfile', path], capture_output=True, text=True, timeout=60)
    if out.returncode != 0:
        print('teq failed:', out.stderr)
        return 2
    ours = [m for m in (teq_member(l) for l in out.stdout.splitlines() if l.startswith('  ')) if m]
    missing = sorted(set(theirs) - set(ours))
    extra = sorted(set(ours) - set(theirs))
    for m in missing:
        print('javap only:', m)
    for m in extra:
        print('teq only:  ', m)
    if missing or extra:
        return 1
    if len(ours) != len(theirs):
        print(f'{len(ours)} members printed, javap prints {len(theirs)}')
        return 1
    print(f'{path}: {len(ours)} members agree with javap')
    return 0


if __name__ == '__main__':
    sys.exit(main())
