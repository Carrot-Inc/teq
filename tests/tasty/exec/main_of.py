# tests/tasty/exec/main_of.py <case>: the class and the static method a case of tests/cases runs as its
# main, where the source names one: `p.S$package m` of `@main def m` in the file S.scala of the
# package p, `p.O main` of an object's `def main`; nothing where it names none or several.
import os, re, sys

p = sys.argv[1]
files = [p] if os.path.isfile(p) else sorted(os.path.join(p, f) for f in os.listdir(p) if f.endswith('.scala'))
found = []
mains = 0
for f in files:
    t = open(f).read()
    pks = re.findall(r'^package\s+([\w.]+)\s*$', t, re.M)
    pre = '.'.join(pks) + '.' if pks else ''
    stem = os.path.basename(f)[:-6].replace('-', '$minus')
    for m in re.finditer(r'^@main\s+def\s+(\w+)', t, re.M):
        found.append(pre + stem + '$package ' + m.group(1))
    for m in re.finditer(r'^object\s+(\w+)[^\n]*\n(?:(?!^\S).*\n)*?\s+def main\(', t, re.M):
        found.append(pre + m.group(1) + ' main')
    mains += len(re.findall(r'\bdef main\(', t))
# A `def main` of a trait or class makes the objects that inherit it mains too.
if len(found) == 1 and mains == sum(1 for x in found if x.endswith(' main')):
    print(found[0])
