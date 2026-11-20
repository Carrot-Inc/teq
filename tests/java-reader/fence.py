#!/usr/bin/env python3
"""tests/java-reader/fence.py <teq> <out> [jobs] [corpus...]: the Java reader's FENCE, every column failing. Every class the reader records (its
record as <teq> prints it under TEQ_JAVA_RECORDS=raw: access, parents, each field's and method's access, name,
descriptor and signature, and the problems it holds for a Scala reader) has, member for member, the descriptor the
oracle's class file has, or the class is refused with a place (a diagnostic told in its file, or a problem held for a
reader at an offset). The oracle: for the JDK its own class files (the runtime image, jimage-extracted); for dotty's
Java tests javac's class files (javac alone, else after scalac over the directory, vulpix's order).
Corpora: jdk (src.zip: java.base, java.xml, java.sql, java.desktop's non-GUI packages; a unit per package, its files
together at their package path, the JDK's other packages from ct.sym), dotty (each directory of dotty's tests at the
tag, $SCALA3_TAG, as dotty-units.tsv beside this file lists them: a unit's .java files together), info, probes
(probes/ beside this file).
COLUMNS, each failing: the parents; the members' descriptors (fields by name, methods by name and descriptor); the
generic signatures of the class and every member, compared after ONE normalization (a member or a class without a
Signature attribute stands for the signature its descriptor or its parents give; a method signature's thrown types,
`^...`, which the reader does not read, are cut); the flags a reader acts on (a class's public, final, interface and
abstract; a field's access, static and final; a method's access, static, final, varargs and abstract); a static final
field's constant (ConstantValue) where the source's initialiser is a lone literal, the reference's reading
(`tryConstant`, `varDecl`); THE INVENTORY from the oracle's own class files (javac's output, a JDK package's class files
of the unit's sources by their SourceFile): each top-level or member class has its record, or a refusal told at its
own declaration (a member class at its owner's), a compact source's classes at its first member; and module-info and
package-info, each run beside scalac's outcome.
LEFT OUT, by the source view: the oracle's bridges, synthetic members and <clinit>; anonymous and local classes.
The JDK is $JAVA_HOME's (else the `java` on the PATH): its src.zip, and its runtime image's class files extracted into
<out>/jdk-classes (or $FENCE_JDK_CLASSES). FENCE_THREADS=n checks at n workers, FENCE_SHUFFLE=<seed> shuffles each unit's
sources and the units, FENCE_LIMIT=n takes the first n units, and FENCE_PLANT=<column> plants a fault of that column in
every record (parents, members, signatures, flags, constants, inventory) or, `info`, turns teq's outcome over in the
module-info and package-info corpus: the check that the column fails.
The whole run (the four corpora, six jobs) takes about ten minutes on a 16-core machine; the gate runs none of it.
-> <out>/fence.tsv (a line per class: corpus, unit, class, verdict, details) and <out>/summary.txt."""
import collections, concurrent.futures, hashlib, json, os, random, re, shutil, subprocess, sys, zipfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import classfile  # noqa: E402
FLAGS = {'class': 0x0611, 'field': 0x001f, 'method': 0x049f}
TEQ, OUT = sys.argv[1], sys.argv[2]
JOBS = int(sys.argv[3]) if len(sys.argv) > 3 and sys.argv[3].isdigit() else 6
WANT = [a for a in sys.argv[3:] if not a.isdigit()] or ['jdk', 'info', 'dotty', 'probes']
THREADS = os.environ.get('FENCE_THREADS', '1')
PLANT = os.environ.get('FENCE_PLANT')
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
JAVA_HOME = os.environ.get('JAVA_HOME') or os.path.dirname(os.path.dirname(os.path.realpath(shutil.which('java') or 'java')))
SRCZIP = os.path.join(JAVA_HOME, 'lib', 'src.zip')
JDKX = os.environ.get('FENCE_JDK_CLASSES') or os.path.join(OUT, 'jdk-classes')
TAG = os.environ.get('SCALA3_TAG') or os.path.join(os.path.dirname(os.environ.get('SCALA3', os.path.join(ROOT, '..', 'scala3'))), 'scala3-3.8.4')
M2 = os.path.expanduser('~/.cache/coursier/v1/https/repo1.maven.org/maven2/')
SL = M2 + 'org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar'
SC = ':'.join(M2 + j for j in [
    'org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar',
    'org/scala-lang/scala3-compiler_3/3.8.4/scala3-compiler_3-3.8.4.jar',
    'org/scala-lang/scala3-interfaces/3.8.4/scala3-interfaces-3.8.4.jar',
    'org/scala-lang/tasty-core_3/3.8.4/tasty-core_3-3.8.4.jar',
    'org/scala-lang/modules/scala-asm/9.9.0-scala-1/scala-asm-9.9.0-scala-1.jar',
    'org/scala-sbt/compiler-interface/1.10.7/compiler-interface-1.10.7.jar'])
GUI = ('java/awt', 'javax/swing', 'sun/awt', 'sun/java2d', 'sun/swing', 'sun/font', 'sun/lwawt', 'com/sun/java/swing',
       'sun/print', 'javax/accessibility', 'sun/applet', 'java/applet')
MAIN = 'object FenceMain { def main(args: Array[String]): Unit = () }\n'
DECL = re.compile(r'^(?:(?:public|protected|private|abstract|final|static|strictfp|sealed|non-sealed)\s+)*(?:@interface|class|interface|enum|record)\s+(\w+)', re.M)


def run(cmd, timeout=300, env=None):
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, env=env, errors='replace')
        return r.returncode, r.stdout, r.stderr
    except subprocess.TimeoutExpired:
        return 'timeout', '', ''


# a literal as the reference's scanner reads one (no `_`: dotty's getNumber stops there, and then no constant is read)
# (and no hex or binary float: the scanner reads a hex literal's digits alone, `0x1` of `0x1.0p3`, and no `;` follows)
LIT = r"""(?:-|!)?\s*(?:'(?:[^'\\]|\\.)+'|"(?:[^"\\]|\\.)*"|true|false|0[xX][0-9a-fA-F]+[lL]?|0[bB][01]+[lL]?|\.?[0-9][0-9.eE+-]*[lLfFdD]?)"""


def records(err, root):
    """The raw records, the sources' declarations (DECL: file, binary, start, name's offset; COMPACT: a compact
    source's first member) and the told diagnostics of a `compiler check` under TEQ_JAVA_RECORDS=raw."""
    recs, cur, diags, decls, compact = {}, None, [], [], {}
    rel = lambda p: os.path.relpath(p, root) if p.startswith(root) else p
    for l in err.splitlines():
        f = l.split('\t')
        if cur is not None:
            cur['raw'].append(l)
        if f[0] == 'RECORD':
            cur = {'raw': [l], 'name': f[1], 'access': int(f[2], 16), 'super': f[3], 'interfaces': [x for x in f[4].split(',') if x],
                   'signature': None if f[5] == '-' else f[5], 'fields': [], 'methods': [], 'held': [], 'constants': {}}
        elif f[0] in ('F', 'M') and cur is not None:
            (cur['fields'] if f[0] == 'F' else cur['methods']).append((int(f[1], 16), f[2], f[3], None if f[4] == '-' else f[4]))
            if f[0] == 'F' and len(f) > 5 and f[5] != '-':
                cur['constants'][f[2]] = f[5]
        elif f[0] == 'HELD' and cur is not None:
            cur['held'].append((int(f[1]), '\t'.join(f[2:])))
        elif f[0] == 'END' and cur is not None:
            cur['digest'] = hashlib.sha1('\n'.join(cur['raw']).encode()).hexdigest()[:12]
            recs[cur['name']] = cur; cur = None
        elif f[0] == 'DECL':
            decls.append((rel(f[1]), f[2], int(f[3]), int(f[4])))
        elif f[0] == 'COMPACT':
            compact[rel(f[1])] = None if f[2] == '-' else int(f[2])
        else:
            m = re.match(r'^(.*?):(\d+):(\d+): (error|warning): (.*)$', l)
            if m and m.group(4) == 'error':
                diags.append((rel(m.group(1)), int(m.group(2)), int(m.group(3)), m.group(5)))
    return recs, diags, decls, compact


def eff_sig(kind, sig, desc=None, rec=None):
    """A signature as compared: the written one (a method's thrown types cut), else the one its descriptor or its
    parents give."""
    if kind == 'class':
        return sig if sig else ''.join(f'L{t};' for t in [rec['super'] or 'java/lang/Object'] + rec['interfaces'])
    s = sig if sig else desc
    return s.split('^')[0] if kind == 'method' else s


def planted(rec):
    """FENCE_PLANT=<column>: the record with a fault of that column planted, as a reader's fault would make it, for the
    check that the column fails: parents (an interface more), members (the first method's descriptor changed),
    signatures (the class's signature changed), flags (final flipped on the class), constants (the first constant
    changed), inventory (the record gone)."""
    rec = dict(rec)
    if PLANT == 'parents':
        rec['interfaces'] = rec['interfaces'] + ['planted/Fault']
    elif PLANT == 'members' and rec['methods']:
        m = rec['methods'][0]; rec['methods'] = [(m[0], m[1], m[2] + 'X', m[3])] + rec['methods'][1:]
    elif PLANT == 'signatures':
        rec['signature'] = 'Lplanted/Fault;'
    elif PLANT == 'flags':
        rec['access'] = rec['access'] ^ 0x0010
    elif PLANT == 'constants' and rec['constants']:
        n = sorted(rec['constants'])[0]; rec['constants'] = dict(rec['constants'], **{n: rec['constants'][n] + '1'})
    elif PLANT == 'inventory':
        return None
    return rec


def compare(rec, ora, text):
    """The differences of a record and its oracle by column: parents, members, signatures, flags, constants."""
    cols = collections.defaultdict(list)
    if (rec['super'] or 'java/lang/Object') != (ora['super'] or 'java/lang/Object') and not (rec['access'] & 0x200):
        cols['parents'].append(f"super {rec['super']} / {ora['super']}")
    if rec['interfaces'] != ora['interfaces']:
        cols['parents'].append(f"interfaces {','.join(rec['interfaces'])} / {','.join(ora['interfaces'])}")
    if eff_sig('class', rec['signature'], rec=rec) != eff_sig('class', ora['signature'], rec=ora):
        cols['signatures'].append(f"class {rec['signature']} / {ora['signature']}")
    if (rec['access'] & FLAGS['class']) != (ora['access'] & FLAGS['class']):
        cols['flags'].append(f"class {rec['access'] & FLAGS['class']:#x}/{ora['access'] & FLAGS['class']:#x}")
    of = {f[1]: f for f in ora['fields'] if not f[0] & classfile.ACC_SYNTHETIC}
    rf = {f[1]: f for f in rec['fields']}
    for n in sorted(set(of) | set(rf)):
        if n not in rf:
            cols['members'].append(f'field {n} missing'); continue
        if n not in of:
            cols['members'].append(f'field {n} extra'); continue
        if rf[n][2] != of[n][2]:
            cols['members'].append(f'field {n} {rf[n][2]} / {of[n][2]}')
        if eff_sig('field', rf[n][3], rf[n][2]) != eff_sig('field', of[n][3], of[n][2]):
            cols['signatures'].append(f'field {n} {rf[n][3]} / {of[n][3]}')
        if (rf[n][0] & FLAGS['field']) != (of[n][0] & FLAGS['field']):
            cols['flags'].append(f"field {n} {rf[n][0] & FLAGS['field']:#x}/{of[n][0] & FLAGS['field']:#x}")
        # a static final field's constant: the record's against the class file's; where the class file has one the
        # record has not, failing only where the source writes a lone literal (what the reference reads)
        if of[n][0] & 0x0018 == 0x0018:
            rc, oc = rec['constants'].get(n), ora['constants'].get(n)
            if rc is not None and rc != oc:
                cols['constants'].append(f'field {n} {rc} / {oc}')
            elif rc is None and oc is not None and re.search(r'\b' + re.escape(n) + r'\s*=\s*' + LIT + r'\s*;', text):
                cols['constants'].append(f'field {n} none / {oc}')
    om = {(m[1], m[2]): m for m in ora['methods'] if not m[0] & (classfile.ACC_SYNTHETIC | classfile.ACC_BRIDGE) and m[1] != '<clinit>'}
    rm = {(m[1], m[2]): m for m in rec['methods']}
    for k in sorted(set(om) | set(rm)):
        if k not in rm:
            cols['members'].append(f'method {k[0]}{k[1]} missing')
        elif k not in om:
            cols['members'].append(f'method {k[0]}{k[1]} extra')
        else:
            if eff_sig('method', rm[k][3], k[1]) != eff_sig('method', om[k][3], k[1]):
                cols['signatures'].append(f'method {k[0]} {rm[k][3]} / {om[k][3]}')
            if (rm[k][0] & FLAGS['method']) != (om[k][0] & FLAGS['method']):
                cols['flags'].append(f"method {k[0]}{k[1]} {rm[k][0] & FLAGS['method']:#x}/{om[k][0] & FLAGS['method']:#x}")
    return cols


def offset_of(text, line, col):
    """A diagnostic's place (its line and column, from 1) as an offset into the text."""
    starts = [0] + [m.end() for m in re.finditer('\n', text)]
    return starts[line - 1] + col - 1 if 0 < line <= len(starts) else -1


def verdicts(corpus, unit, root, srcs, recs, diags, decls, compact, classes):
    """A row per class of the oracle's output (`classes`: binary name -> its class file read), and per record the oracle
    has no class for."""
    rows = []
    texts = {s: open(os.path.join(root, s), errors='replace').read() for s in srcs}
    by_base = collections.defaultdict(list)
    for s in srcs:
        by_base[os.path.basename(s)].append(s)
    # each told refusal owned by the top-level declaration it stands in (the last that starts at or before it)
    owned = collections.defaultdict(list)
    loose = collections.defaultdict(list)
    for (file, line, col, msg) in diags:
        if not file.endswith('.java') or file not in texts:
            continue
        at = offset_of(texts[file], line, col)
        mine = [d for d in decls if d[0] == file and d[2] <= at]
        (owned[max(mine, key=lambda d: d[2])[1]] if mine else loose[file]).append(f'{file}:{line}:{col}: {msg}')
    done = set()
    for b, ora in sorted(classes.items()):
        top = b.split('$')[0]
        simple = b.rsplit('/', 1)[-1]
        if '$' in simple and re.match(r'\d', simple.split('$')[1]):
            continue  # an anonymous or local class: no declaration a reader sees
        srcfile = next(iter(by_base.get(ora.get('source') or '', [])), None)
        done.add(b)
        if srcfile in compact:
            told = loose.get(srcfile, [])
            rows.append([corpus, unit, b, 'refused' if told else 'NOT RECORDED', (told[0] if told else 'a compact source: no refusal told')[:220]])
            continue
        if b != top:
            told = owned.get(top, []) + ([f'held: {m}' for _, m in recs[top]['held']] if top in recs else [])
            rows.append([corpus, unit, b, 'refused' if told else 'NOT RECORDED', (told[0] if told else 'a member class without its refusal')[:220]])
            continue
        rec = recs.get(b)
        told = owned.get(b, []) + ([f'{srcfile}@{o}: {m}' for o, m in rec['held']] if rec else [])
        if told:
            rows.append([corpus, unit, b, 'refused', told[0][:220]]); continue
        if rec is None:
            rows.append([corpus, unit, b, 'NOT RECORDED', 'no record and no refusal']); continue
        if PLANT:
            rec = planted(rec)
            if rec is None:
                rows.append([corpus, unit, b, 'NOT RECORDED', 'planted: the record dropped']); continue
        cols = compare(rec, ora, texts.get(srcfile, ''))
        rows.append([corpus, unit, b, 'DIFFERS ' + ','.join(sorted(cols)) if cols else 'equal', ('; '.join(f'{c}: ' + '; '.join(v) for c, v in sorted(cols.items())) or '-')[:500]])
    for b in sorted(recs):
        if b not in done:
            # the JDK's image lacks some classes its src.zip holds (another system's, generated at its link): no oracle
            rows.append([corpus, unit, b, 'no oracle' if corpus == 'jdk' else 'NO ORACLE', 'a record of no class the oracle has'])
    for r in rows:
        r.append(recs[r[2]]['digest'] if r[2] in recs else '-')
    return rows


def ordered(root, rels, unit):
    """A unit's files as the command line names them: sorted, or shuffled (FENCE_SHUFFLE=<seed>, by the unit)."""
    files = sorted(os.path.join(root, r) for r in rels)
    if os.environ.get('FENCE_SHUFFLE'):
        random.Random(f"{os.environ['FENCE_SHUFFLE']}:{unit}").shuffle(files)
    return files


def jdk_units():
    """A unit per package of the four modules: its source files together at their package path."""
    z = zipfile.ZipFile(SRCZIP)
    groups = collections.defaultdict(list)
    for n in sorted(z.namelist()):
        if not n.endswith('.java') or n.endswith(('module-info.java', 'package-info.java')) or '/snippet-files/' in n:
            continue
        mod, rest = n.split('/', 1)
        pkg = rest.rsplit('/', 1)[0] if '/' in rest else ''
        if mod in ('java.base', 'java.xml', 'java.sql') or (mod == 'java.desktop' and not pkg.startswith(GUI)):
            groups[(mod, pkg)].append(n)
    return [('jdk', f'{m}/{p}', names) for (m, p), names in sorted(groups.items())]


def dotty_units():
    """The units of dotty-units.tsv beside this file, at the tag; a unit that is a whole category's flat directory
    (tests/pos, tests/neg, ...: unrelated single-file tests) split into a unit per file."""
    out = []
    for l in open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'dotty-units.tsv')):
        if l.startswith('#'):
            continue
        ident, rel, fs = l.rstrip('\n').split('\t')
        files = [os.path.join(TAG, f) for f in fs.split(',')]
        if rel.count('/') == 1 and len(files) > 1:
            out += [('dotty', f"{ident}:{os.path.basename(f)}", os.path.dirname(f), [f]) for f in files]
        else:
            out.append(('dotty', ident, os.path.join(TAG, rel), files))
    seen, uniq = set(), []
    for u in out:
        if u[1] not in seen:
            seen.add(u[1]); uniq.append(u)
    return uniq


def one_jdk(u):
    _, n, names = u
    mod = n.split('/', 1)[0]
    w = os.path.join(OUT, 'w', n.replace('/', '_')); shutil.rmtree(w, ignore_errors=True)
    rels = []
    with zipfile.ZipFile(SRCZIP) as z:
        for f in names:
            rest = f.split('/', 1)[1]
            os.makedirs(os.path.join(w, os.path.dirname(rest)), exist_ok=True)
            open(os.path.join(w, rest), 'wb').write(z.read(f)); rels.append(rest)
    open(os.path.join(w, 'FenceMain.scala'), 'w').write(MAIN)
    e, so, err = run([TEQ, 'compiler', 'check'] + ordered(w, rels + ['FenceMain.scala'], n) + ['--target', 'jvm', '--std', 'scala-library', '--classpath', SL, '--no-cache'],
                     env=dict(os.environ, TEQ_JAVA_RECORDS='raw', TEQ_THREADS=THREADS))
    recs, diags, decls, compact = records(err, w)
    # the oracle's output for the unit: the package's class files whose SourceFile is one of the unit's sources
    pkgdir = os.path.join(JDKX, mod, n.split('/', 1)[1] if '/' in n else '')
    bases = {os.path.basename(r) for r in rels}
    classes = {}
    for f in sorted(os.listdir(pkgdir)) if os.path.isdir(pkgdir) else []:
        if f.endswith('.class') and f not in ('module-info.class', 'package-info.class'):
            c = classfile.read(open(os.path.join(pkgdir, f), 'rb').read())
            if c.get('source') in bases:
                classes[c['name']] = c
    rows = verdicts('jdk', n, w, rels, recs, diags, decls, compact, classes)
    if e == 'timeout':
        rows.append(['jdk', n, '-', 'timeout', '-', '-'])
    shutil.rmtree(w, ignore_errors=True)
    return rows


def info_units():
    """module-info.java and package-info.java of the four modules, each its own unit."""
    z = zipfile.ZipFile(SRCZIP)
    out = []
    for n in sorted(z.namelist()):
        if n.endswith(('module-info.java', 'package-info.java')):
            mod, rest = n.split('/', 1)
            pkg = rest.rsplit('/', 1)[0] if '/' in rest else ''
            if mod in ('java.base', 'java.xml', 'java.sql') or (mod == 'java.desktop' and not pkg.startswith(GUI)):
                out.append(('info', n, [n]))
    return out


def one_info(u):
    """A module-info or package-info source alone beside a Scala main: teq's outcome against scalac's."""
    _, n, names = u
    w = os.path.join(OUT, 'w', 'info_' + n.replace('/', '_')); shutil.rmtree(w, ignore_errors=True)
    rest = n.split('/', 1)[1]
    with zipfile.ZipFile(SRCZIP) as z:
        os.makedirs(os.path.join(w, os.path.dirname(rest)), exist_ok=True)
        open(os.path.join(w, rest), 'wb').write(z.read(n))
    open(os.path.join(w, 'FenceMain.scala'), 'w').write(MAIN)
    e, so, err = run([TEQ, 'compiler', 'check', w, '--target', 'jvm', '--std', 'scala-library', '--classpath', SL, '--no-cache'],
                     env=dict(os.environ, TEQ_THREADS=THREADS))
    te = next((l for l in err.splitlines() if ': error: ' in l), '').replace(w + '/', '')
    sd = os.path.join(w, 'scalac'); os.makedirs(sd)
    se, sso, serr = run(['java', '-XX:-UsePerfData', '-Xmx1g', '-cp', SC, 'dotty.tools.dotc.Main', '-color:never', '-classpath', SL, '-d', sd,
                         os.path.join(w, rest), os.path.join(w, 'FenceMain.scala')], 300)
    sx = next((l for l in (sso + serr).splitlines() if 'Error' in l or 'error' in l), '').replace(w + '/', '')
    if PLANT == 'info':
        e = 0 if e else 1  # the planted fault of this corpus: teq's outcome turned over
    same = (e == 0) == (se == 0)
    shutil.rmtree(w, ignore_errors=True)
    verdict = ('as scalac: ' + ('accepted' if e == 0 else 'refused')) if same else 'DIFFERS outcome'
    return [['info', n, rest, verdict, f'teq {e} {te[:120]} | scalac {se} {sx[:120]}', '-']]


def probe_units():
    """The claim's programs (probes/): each an import of a package under the JDK's roots that ct.sym does not hold,
    beside a class of the name a later scope would give (a package-mate, an on-demand import's, an overload taking
    Object); each to be refused where the name is used, never built."""
    d = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'probes')
    return [('probes', n, os.path.join(d, n)) for n in sorted(os.listdir(d))]


def one_probe(u):
    _, n, d = u
    w = os.path.join(OUT, 'w', 'probe_' + n); shutil.rmtree(w, ignore_errors=True); os.makedirs(w)
    srcs = sorted(os.path.join(dp, f) for dp, _, fs in os.walk(d) for f in fs if f.endswith(('.java', '.scala')))
    e, so, err = run([TEQ, 'compiler', 'build'] + srcs + ['--target', 'jvm', '-o', os.path.join(w, 'out'), '--classpath', SL],
                     env=dict(os.environ, TEQ_THREADS=THREADS))
    errs = [l.replace(d + '/', '') for l in (so + err).splitlines() if ': error: ' in l]
    at_use = [l for l in errs if 'J.java' in l and 'Not found: type Unsafe' in l]
    verdict = 'refused at the use' if e != 0 and at_use else 'BUILT' if e == 0 else 'REFUSED ELSEWHERE'
    shutil.rmtree(w, ignore_errors=True)
    return [['probes', n, n, verdict, '; '.join(errs)[:220] or '-', '-']]


def one_dotty(u):
    _, ident, path, files = u
    root = path if os.path.isdir(path) else os.path.dirname(path)
    w = os.path.join(OUT, 'w', ident); shutil.rmtree(w, ignore_errors=True); os.makedirs(w)
    srcs, srcs_scala = [], []
    for f in files:
        rel = os.path.relpath(f, root); dest = os.path.join(w, 'src', rel)
        os.makedirs(os.path.dirname(dest), exist_ok=True); shutil.copyfile(f, dest); srcs.append(rel)
    # the unit's own Scala sources, as the oracle compiles them (a directory's; a flat directory's file its stem's)
    if ':' in ident:
        stem = os.path.join(root, os.path.splitext(os.path.basename(files[0]))[0] + '.scala')
        own = [stem] if os.path.exists(stem) else []
    else:
        own = [os.path.join(dp, f) for dp, _, fs in os.walk(root) for f in fs if f.endswith('.scala')] if os.path.isdir(root) else []
    for f in own:
        rel = os.path.relpath(f, root); dest = os.path.join(w, 'src', rel)
        os.makedirs(os.path.dirname(dest), exist_ok=True); shutil.copyfile(f, dest); srcs_scala.append(rel)
    open(os.path.join(w, 'src', 'FenceMain.scala'), 'w').write(MAIN)
    sroot = os.path.join(w, 'src')
    e, so, err = run([TEQ, 'compiler', 'check'] + ordered(sroot, srcs + srcs_scala + ['FenceMain.scala'], ident) + ['--target', 'jvm', '--std', 'scala-library', '--classpath', SL, '--no-cache'],
                     env=dict(os.environ, TEQ_JAVA_RECORDS='raw', TEQ_THREADS=THREADS))
    recs, diags, decls, compact = records(err, sroot)
    # the oracle: javac alone, else scalac over the directory's sources (its .scala files at the tag) then javac
    jd = os.path.join(w, 'javac'); os.makedirs(jd)
    java = [os.path.join(sroot, s) for s in srcs]
    je, _, jerr = run(['javac', '-J-XX:-UsePerfData', '-encoding', 'UTF-8', '-nowarn', '-cp', SL, '-d', jd] + java)
    how = 'javac'
    if je != 0:
        if ':' in ident:
            # a single-file test of a flat directory: its own Scala file of the same stem, where there is one
            stem = os.path.join(root, os.path.splitext(os.path.basename(files[0]))[0] + '.scala')
            scala = [stem] if os.path.exists(stem) else []
        else:
            scala = [os.path.join(dp, f) for dp, _, fs in os.walk(root) for f in fs if f.endswith('.scala')] if os.path.isdir(root) else []
        sd = os.path.join(w, 'scalac'); os.makedirs(sd)
        se, _, _ = run(['java', '-XX:-UsePerfData', '-Xmx1g', '-cp', SC, 'dotty.tools.dotc.Main', '-color:never', '-classpath', SL, '-d', sd] + scala + java, 600)
        je, _, jerr = run(['javac', '-J-XX:-UsePerfData', '-encoding', 'UTF-8', '-nowarn', '-cp', f'{sd}:{SL}', '-d', jd] + java)
        how = 'scalac+javac' if je == 0 else 'none'
    classes = {}
    for dp, _, fs in os.walk(jd):
        for f in fs:
            if f.endswith('.class') and f not in ('module-info.class', 'package-info.class'):
                c = classfile.read(open(os.path.join(dp, f), 'rb').read())
                classes[c['name']] = c
    rows = verdicts('dotty', ident, sroot, srcs, recs, diags, decls, compact, classes)
    if how == 'none':
        # no oracle: javac refuses the unit (alone and after scalac); each record listed, judged by nothing
        rows = [r[:3] + ['no oracle', '-', r[5]] for r in rows] or [['dotty', ident, '-', 'no oracle', '-', '-']]
    for r in rows:
        r[4] = f'[{how}] ' + r[4]
    shutil.rmtree(w, ignore_errors=True)
    return rows


units = (jdk_units() if 'jdk' in WANT else []) + (info_units() if 'info' in WANT else []) + (dotty_units() if 'dotty' in WANT else []) + (probe_units() if 'probes' in WANT else [])
if os.environ.get('FENCE_SHUFFLE'):
    import random
    random.Random(int(os.environ['FENCE_SHUFFLE'])).shuffle(units)
lim = os.environ.get('FENCE_LIMIT')
if lim:
    units = units[:int(lim)]
os.makedirs(OUT, exist_ok=True)
# the JDK's class files, the jdk corpus's oracle: the runtime image's modules the corpus reads, extracted once
if 'jdk' in WANT and not os.path.isdir(JDKX):
    subprocess.run([os.path.join(JAVA_HOME, 'bin', 'jimage'), 'extract', '--dir', JDKX, '--include',
                    'regex:/(java.base|java.xml|java.sql|java.desktop)/.*', os.path.join(JAVA_HOME, 'lib', 'modules')], check=True)
# the scala-library jar as the tree finds it
SL = subprocess.run(['bash', '-c', '. tests/support/jars.sh; jar_of scala-library'], capture_output=True, text=True, cwd=ROOT).stdout.strip() or SL
with concurrent.futures.ThreadPoolExecutor(JOBS) as ex:
    rows = [r for rs in ex.map(lambda u: one_jdk(u) if u[0] == 'jdk' else one_info(u) if u[0] == 'info' else one_probe(u) if u[0] == 'probes' else one_dotty(u), units) for r in rs]
rows.sort(key=lambda r: (r[0], r[1], r[2]))
with open(os.path.join(OUT, 'fence.tsv'), 'w') as fh:
    fh.write('# corpus\tunit\tclass\tverdict (equal, DIFFERS <columns>, refused, NOT RECORDED, NO ORACLE, no oracle, as scalac, DIFFERS outcome)\tdetails\trecord digest\n')
    for r in rows:
        fh.write('\t'.join(r) + '\n')
summ = []
for c in WANT:
    rc = [r for r in rows if r[0] == c]
    v = collections.Counter(r[3].split(' ')[0] if r[3].startswith('DIFFERS') and r[0] != 'info' else r[3] for r in rc)
    colc = collections.Counter(c for r in rc if r[3].startswith('DIFFERS ') and r[0] != 'info' for c in r[3].split(' ', 1)[1].split(','))
    summ.append(f'{c}: units {len(set(r[1] for r in rc))}, classes {len(rc)}: ' + ', '.join(f'{k} {n}' for k, n in v.most_common()))
    summ.append('  DIFFERS by column: ' + (', '.join(f'{k} {n}' for k, n in colc.most_common()) or 'none'))
    first = collections.Counter(re.sub(r'^\S+?(:\d+:\d+|@\d+): ', '', r[4].split('] ', 1)[-1]) for r in rc if r[3] in ('refused', 'NOT RECORDED'))
    kinds = collections.Counter(re.sub(r'(type|of|naming|parent|member type|finds) [\w.$]+', r'\1 X', k)[:100] for k in first.elements())
    summ.append('  refusals by kind:')
    summ += [f'    {n}\t{k}' for k, n in kinds.most_common(40)]
open(os.path.join(OUT, 'summary.txt'), 'w').write('\n'.join(summ) + '\n')
print('\n'.join(summ[:3]))
