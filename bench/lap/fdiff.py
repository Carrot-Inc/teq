# fdiff.py <master.fn> <landing.fn> <n> [grep]: cachegrind per-function instructions, landing against master
import re, sys, collections
def load(p):
    c = collections.Counter(); tot = 0; on = False
    for line in open(p):
        if 'PROGRAM TOTALS' in line:
            tot = int(line.split()[0].replace(',', ''))
        if 'Function:file summary' in line: on = True; continue
        if not on: continue
        m = re.match(r'[>\s]\s*([\d,]+)\s+(.*)$', line)
        if not m: continue
        fn = m.group(2)
        fn = re.sub(r':(\?\?\?|\./.*|/.*)$', '', fn)
        c[fn] += int(m.group(1).replace(',', ''))
    return c, tot
if __name__ == '__main__':
    m, tm = load(sys.argv[1]); l, tl = load(sys.argv[2]); n = int(sys.argv[3])
    pat = sys.argv[4] if len(sys.argv) > 4 else None
    print(f'master {tm:,} landing {tl:,} delta {tl-tm:+,} ({(tl/tm-1)*100:+.2f}%)')
    keys = [f for f in set(m)|set(l) if not pat or re.search(pat, f)]
    for fn in sorted(keys, key=lambda f: -abs(l[f]-m[f]))[:n]:
        print(f'{l[fn]-m[fn]:+13,} {(l[fn]-m[fn])/tm*100:+.3f}%  m {m[fn]:>12,} l {l[fn]:>12,}  {fn[:125]}')
