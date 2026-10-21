# bench/fix/summary.py [labels...]: out/measure/results.tsv (bench/fix/run.sh's): results.tsv as tables, one row per label, corpus, settings, workers and temperature.
import sys, re, collections
rows = [l.rstrip('\n').split('\t') for l in open('out/measure/results.tsv')]
want = sys.argv[1:]
def phase(p, name):
    m = re.search(name + r' (\d+) ms', p)
    return m.group(1) if m else '-'
print('| label | corpus | settings | workers | run | exit | wall s | peak KiB | digest | typing ms | rules ms | docs ms | longest ms | planning ms |')
print('|---|---|---|---|---|---|---|---|---|---|---|---|---|---|')
for r in rows:
    if len(r) < 10 or (want and r[0] not in want): continue
    label, corpus, conf, w, temp, code, wall, rss, digest, p = r[:10]
    docs = re.search(r'their documents (\d+) ms, the longest (\d+) ms', p)
    print('| ' + ' | '.join([label, corpus, conf, w, temp, code, wall, rss, digest, phase(p, 'typing'), phase(p, 'the rules'), docs.group(1) if docs else '-', docs.group(2) if docs else '-', phase(p, 'planning')]) + ' |')
