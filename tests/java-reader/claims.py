#!/usr/bin/env python3
"""tests/java-reader/claims.py <teq> <base teq> <out>: each program of claims/ beside this file (an import's or a name's
resolution in a Java source) by scalac 3.8.4 (its .java and Main.scala together) and javac (its .java against
scalac's output, else against scala-library alone), then by the two binaries: `compiler build --target jvm`, javac
over the .java against the build, `java Main`. -> <out>/claims.tsv, a line per program: scalac's, javac's, the
binary's and the base's; and the count of programs whose verdict (built, or refused) is scalac's on each binary."""
import glob, os, shutil, subprocess, sys
SRC = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'claims')
OUT = sys.argv[3]
M2 = os.path.expanduser('~/.cache/coursier/v1/https/repo1.maven.org/maven2/')
SL = M2 + 'org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar'
SC = ':'.join(M2 + j for j in [
    'org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar',
    'org/scala-lang/scala3-compiler_3/3.8.4/scala3-compiler_3-3.8.4.jar',
    'org/scala-lang/scala3-interfaces/3.8.4/scala3-interfaces-3.8.4.jar',
    'org/scala-lang/tasty-core_3/3.8.4/tasty-core_3-3.8.4.jar',
    'org/scala-lang/modules/scala-asm/9.9.0-scala-1/scala-asm-9.9.0-scala-1.jar',
    'org/scala-sbt/compiler-interface/1.10.7/compiler-interface-1.10.7.jar'])
HEADS = [('binary', sys.argv[1]), ('base', sys.argv[2])]
def run(cmd, timeout=300):
    try:
        r = subprocess.run([str(c) for c in cmd], capture_output=True, text=True, timeout=timeout, errors='replace')
        return r.returncode, (r.stdout + r.stderr).strip()
    except subprocess.TimeoutExpired:
        return 'timeout', ''
def first(t, d):
    l = next((l for l in t.splitlines() if 'rror' in l and not l.startswith('1 error')), t[:120])
    return l.replace(d + '/', '').strip()[:170]
rows = []
for name in sorted(os.listdir(SRC)):
    d = os.path.join(SRC, name)
    srcs = sorted(glob.glob(d + '/**/*.java', recursive=True) + glob.glob(d + '/*.scala'))
    java = [s for s in srcs if s.endswith('.java')]
    w = os.path.join(OUT, 'w', name); shutil.rmtree(w, ignore_errors=True); os.makedirs(w)
    sd = os.path.join(w, 'scalac'); os.makedirs(sd)
    e, t = run(['java', '-XX:-UsePerfData', '-Xmx1g', '-cp', SC, 'dotty.tools.dotc.Main', '-color:never', '-classpath', SL, '-d', sd] + srcs)
    scalac = 'accepts' if e == 0 else f'refuses: {first(t, d)}'
    jd = os.path.join(w, 'javac'); os.makedirs(jd)
    e2, t2 = run(['javac', '-J-XX:-UsePerfData', '-nowarn', '-cp', f'{sd}:{SL}', '-d', jd] + java)
    javac = 'accepts' if e2 == 0 else f'refuses: {first(t2, d)}'
    if e == 0 and e2 == 0:
        e3, t3 = run(['java', '-XX:-UsePerfData', '-cp', f'{sd}:{jd}:{SL}', 'Main'], 60)
        scalac += f'; runs {e3}: {t3.splitlines()[0][:60] if t3 else ""}'
    row = [name, scalac, javac]
    for tag, b in HEADS:
        o = os.path.join(w, tag); os.makedirs(o)
        e, t = run([b, 'compiler', 'build'] + srcs + ['--no-cache', '--std', 'scala-library', '--classpath', SL, '--target', 'jvm', '--all-mains', '-o', o])
        if e != 0:
            row.append(f'refuses: {first(t, d)}'); continue
        e2, t2 = run(['javac', '-J-XX:-UsePerfData', '-nowarn', '-cp', f'{o}:{SL}', '-d', o] + java)
        if e2 != 0:
            row.append(f'builds; javac refuses: {first(t2, d)}'); continue
        e3, t3 = run(['java', '-XX:-UsePerfData', '-cp', f'{o}:{SL}', 'Main'], 60)
        row.append(f'builds; runs {e3}: ' + ' | '.join(t3.splitlines()[:2])[:120])
    rows.append(row)
    shutil.rmtree(w, ignore_errors=True)
with open(os.path.join(OUT, 'claims.tsv'), 'w') as fh:
    fh.write('# program\tscalac (and its run)\tjavac against scalac\tthe binary\tthe base\n')
    for r in rows: fh.write('\t'.join(r) + '\n')
verdict = lambda x: 'refused' if x.startswith('refuses') else 'built' if x.startswith(('accepts', 'builds')) else x
for i, tag in ((3, 'the binary'), (4, 'the base')):
    same = [r for r in rows if verdict(r[1]) == verdict(r[i])]
    print(f'claims, {tag}: {len(same)} of {len(rows)} as scalac by verdict; apart: ' + (', '.join(r[0] for r in rows if r not in same) or 'none'))
