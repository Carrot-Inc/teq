#!/usr/bin/env python3
"""census.py: over tests/init-view's outputs (out/init-view, run from the repository's root), the fields of the
aligned lines that differ between scalac's dump and teq's view, counted by tree kind and field, an example of each;
the class lines apart. What tests/init-view-full.sh's differing programs differ in."""
import sys, difflib, glob, os, re, collections
sys.path.insert(0, 'tests/init-view')
import compare as c
cnt=collections.Counter(); ex={}; classdiff=0; classtot=0
FIELD=re.compile(r' (\w+)=(\S+(?: \{[^}]*\})?)')
for d in sorted(glob.glob('out/init-view/teq/*/')):
    n=os.path.basename(d.rstrip('/'))
    o=max([x for x in glob.glob(f'out/init-view/oracle/{n}-*') if re.fullmatch(re.escape(n)+r'-[0-9a-f]{16}', os.path.basename(x))], key=os.path.getmtime)
    A=open(o+'/pickler.dump').read().splitlines(); B=open(d+'pickler.dump').read().splitlines()
    ca=[x for x in A if x.startswith('class ')]; cb=[y for y in B if y.startswith('class ')]
    classtot+=len(ca); classdiff+=len(set(ca)^set(cb))
    a=[x for x in A if not x.startswith('class ')]; b=[y for y in B if not y.startswith('class ')]
    sm=difflib.SequenceMatcher(None,[c.head(x) for x in a],[c.head(y) for y in b],autojunk=False)
    for tag,i1,i2,j1,j2 in sm.get_opcodes():
        if tag!='equal': continue
        for x,y in zip(a[i1:i2],b[j1:j2]):
            if x==y: continue
            m=c.POS.match(x); kind=m.group(2) if m else x.split(' ')[0]
            fx=dict((k,v) for k,v in re.findall(r' (\w+)=(\S+)',x)); fy=dict((k,v) for k,v in re.findall(r' (\w+)=(\S+)',y))
            # flags in braces
            bx=re.findall(r'\{[^}]*\}',x); by=re.findall(r'\{[^}]*\}',y)
            keys=set(fx)|set(fy)
            for k in keys:
                if fx.get(k)!=fy.get(k):
                    kk=(kind,k); cnt[kk]+=1; ex.setdefault(kk,(n,x.strip()[:130],y.strip()[:130]))
            if bx!=by:
                kk=(kind,'flags'); cnt[kk]+=1; ex.setdefault(kk,(n,x.strip()[:130],y.strip()[:130]))
print('class lines: scalac',classtot,'differing (sym diff)',classdiff)
for k,v in cnt.most_common(40):
    print(v,k); e=ex[k]; print('     ',e[0],'S:',e[1]); print('      ','  T:',e[2])
