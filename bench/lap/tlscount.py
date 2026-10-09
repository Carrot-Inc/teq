# tlscount.py <callgrind out with --dump-instr> <addresses and functions> <out>: the executions of
# the listed instructions, in all and by function.
import collections, sys
fnof = {}
for l in open(sys.argv[2]):
    a, f = l.split(None, 1)
    fnof[int(a, 16)] = f.strip()
byfn = collections.Counter(); total = 0; cur = 0; skip = False; npos = 1
def step(tok, cur):
    if tok.startswith('0x'): return int(tok, 16)
    if tok.startswith('+'): return cur + int(tok[1:])
    if tok.startswith('-'): return cur - int(tok[1:])
    if tok == '*': return cur
    return int(tok)
for line in open(sys.argv[1]):
    if line.startswith('positions:'):
        npos = len(line.split()) - 1
        continue
    if line.startswith('calls='):
        skip = True
        continue
    parts = line.split()
    if not parts or parts[0][0] not in '0123456789+-*':
        continue
    cur = step(parts[0], cur)
    if skip:
        skip = False
        continue
    if cur in fnof and len(parts) > npos:
        total += int(parts[npos]); byfn[fnof[cur]] += int(parts[npos])
print('thread-pointer reads', total)
with open(sys.argv[3], 'w') as out:
    for f, c in byfn.most_common():
        out.write(f'{c} {f}\n')
