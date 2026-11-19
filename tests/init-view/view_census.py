#!/usr/bin/env python3
"""view_census.py: the view alone. Over the init-view programs and the corpora under out/init-view-corpus (run from the
repository's root), the lines in which teq's view of scalac's OWN pickles (scalac-view.dump) differs from scalac's
InitDump, the lines replaced in place counted by tree kind and differing fields (POS for the position), an example of
each. Every such line is the view's, not the writer's."""
import sys, difflib, glob, os, re, collections

def pairs():
    for d in sorted(glob.glob('out/init-view/teq/*/')):
        n = os.path.basename(d.rstrip('/'))
        os_ = [x for x in glob.glob(f'out/init-view/oracle/{n}-*') if re.fullmatch(re.escape(n) + r'-[0-9a-f]{16}', os.path.basename(x))]
        if os_ and os.path.exists(d + 'scalac-view.dump'):
            yield n, max(os_, key=os.path.getmtime) + '/pickler.dump', d + 'scalac-view.dump'
    for d in sorted(glob.glob('out/init-view-corpus/*/*/')):
        if os.path.exists(d + 'scalac-view.dump') and os.path.exists(d + 'oracle/pickler.dump'):
            yield os.path.basename(d.rstrip('/')), d + 'oracle/pickler.dump', d + 'scalac-view.dump'

if __name__ == '__main__':
    cnt = collections.Counter(); ex = {}; shape = 0
    for n, o, v in pairs():
        A = open(o).read().splitlines(); B = open(v).read().splitlines()
        for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, A, B, autojunk=False).get_opcodes():
            if tag == 'equal':
                continue
            if tag == 'replace' and i2 - i1 == j2 - j1:
                for x, y in zip(A[i1:i2], B[j1:j2]):
                    fx = dict(re.findall(r' (\w+)=(\S+)', x)); fy = dict(re.findall(r' (\w+)=(\S+)', y))
                    k = [f for f in set(fx) | set(fy) if fx.get(f) != fy.get(f)]
                    px = re.search(r'\[[^\]]*\]', x); py = re.search(r'\[[^\]]*\]', y)
                    if px and py and px.group(0) != py.group(0):
                        k.append('POS')
                    if re.findall(r'\{[^}]*\}', x) != re.findall(r'\{[^}]*\}', y):
                        k.append('flags')
                    key = (x.strip().split(' ')[0], tuple(sorted(k)))
                    cnt[key] += 1
                    ex.setdefault(key, (n, x.strip()[:140], y.strip()[:140]))
            else:
                shape += max(i2 - i1, j2 - j1)
    for k, v in cnt.most_common(int(sys.argv[1]) if len(sys.argv) > 1 else 25):
        e = ex[k]
        print(v, k); print('    ', e[0], 'S:', e[1]); print('     T:', e[2])
    print('lines in other hunks (shape):', shape)
