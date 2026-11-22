#!/usr/bin/env python3
"""tests/java-reader/judge.py <master teq> <head teq> [--jobs J]: writes tests/java-reader/judged.tsv, the judges of
progress.py's inputs (corpus.py's), a line an input:
  key, master's verdict with the files named (`compiler check J.java Main.scala --target jvm`: builds, or refuses),
  scalac 3.8.4's over the same two files (accepts, `refuses: <line>:<column>: <its first error>`, or `crashes: <the
  exception>`), scalac's errors (`<id>@<file>:<line>:<column>`, by commas), the listed class, and the reached twin.
A row the head builds and scalac refuses is listed where scalac's own errors say so, never by a file's name or a list:
  - "annotation names" or "unreached class's names": scalac's errors hold a MissingIdent (E006) or a NotAMember (E008),
    and EVERY error scalac gives stands in the Java source, at a place of one of these kinds: in an annotation ("annotation names":
    the reader resolves no annotation's name, reached or not), or in a parent, a member's signature or an import
    ("unreached class's names": the reader resolves the names of a class a Scala source reaches, scalac every
    declaration's);
  - "scalac crashes": scalac ends by an exception before any error; javac judges then, and the tag says what it says;
  - "javac accepts": none of those, every error scalac gives stands in the Java source, and javac compiles it alone: a
    Java source's validity is javac's, where scalac reading it from source refuses what javac compiles.
The reached twin of a listed row: the same Java file and a Scala file in its package that reaches the class the place
stands in (the first class, for an import; where the file declares nothing, its imports alone, the file with a `class
TeqReached {}` after them):
`<package>|<class>|<type parameters>|<line>:<column>`, the place 1-based. scalac and javac run with a 512 MB stack
(the reviewer's 2,000-deep sources).
progress.py reads it all: a row the head builds where scalac refuses fails the suite unless listed; a listed row the
head no longer builds fails it (the list is stale); the twin of an "unreached class's names" row is to be refused at
the place, of an "annotation names" row to build; a "scalac crashes" row is to build where javac accepts; one master
refuses and scalac accepts is a turned row.
Run it by hand when the inputs change (dotty's tag, corpus.py, tests/java-reader/broken): some five minutes on a
16-core machine (one scalac JVM a job, judge/Judge.java). It needs javac, scala-library's and scalac 3.8.4's jars in
the coursier cache, master's binary and the head's."""
import concurrent.futures, os, re, shutil, subprocess, sys, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import corpus

M2 = os.environ.get('COURSIER_CACHE', os.path.expanduser('~/.cache/coursier/v1')) + '/https/repo1.maven.org/maven2/'
SL = M2 + 'org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar'
SC = ':'.join(M2 + j for j in [
    'org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar',
    'org/scala-lang/scala3-compiler_3/3.8.4/scala3-compiler_3-3.8.4.jar',
    'org/scala-lang/scala3-interfaces/3.8.4/scala3-interfaces-3.8.4.jar',
    'org/scala-lang/tasty-core_3/3.8.4/tasty-core_3-3.8.4.jar',
    'org/scala-lang/modules/scala-asm/9.9.0-scala-1/scala-asm-9.9.0-scala-1.jar',
    'org/scala-sbt/compiler-interface/1.10.7/compiler-interface-1.10.7.jar'])
NAMES = ('MissingIdentID', 'NotAMemberID')
JTOKEN = re.compile(r'"""(?:.|\n)*?"""|"(?:\\.|[^"\\\n])*"|\'(?:\\.|[^\'\\\n])*\'|/\*(?:.|\n)*?\*/|//[^\n]*|[A-Za-z_$][\w$]*|\S', re.S)
TYPE_WORDS = ('class', 'interface', 'enum', 'record')


def offset_of(text, line, column):
    """scalac's place (the line 1-based, the column 0-based) as an offset of `text`."""
    lines = text.split('\n')
    return sum(len(l) + 1 for l in lines[:line - 1]) + column if 0 < line <= len(lines) else -1


def places(text):
    """The Java source's tokens, each (start, end, context, class): the context 'import', 'annotation', 'parent',
    'signature' (a member's header, a type's parameters) or 'other' (a body, an initializer, anything else); the class
    the top-level type it stands in, (package, name, type parameters' count), the first one before any type."""
    toks = [(m.start(), m.end(), m.group()) for m in JTOKEN.finditer(text) if not m.group().startswith(('//', '/*'))]
    words = [t for _, _, t in toks]
    package, tops, out = '', [], []
    stack = []            # 'type' or 'block', a brace each
    header = None         # 'name', then 'parents' once extends/implements/permits stands, until the body's `{`
    in_import = annotation = init = False
    ann_parens = 0
    i = 0
    while i < len(toks):
        s, e, t = toks[i]
        prev = words[i - 1] if i else ''
        nxt = words[i + 1] if i + 1 < len(words) else ''
        if not stack and t == 'package' and not tops:
            j, name = i + 1, []
            while j < len(words) and words[j] != ';':
                name.append(words[j]); j += 1
            package = ''.join(name)
        if not stack and t == 'import':
            in_import = True
        # `@interface` declares a type as `interface` does (an annotation declaration): its elements are signatures
        if t in TYPE_WORDS and (prev not in ('.', '@') or (t == 'interface' and prev == '@')) and re.match(r'[A-Za-z_$]', nxt) and (t != 'record' or (i + 2 < len(words) and words[i + 2] in ('(', '<'))):
            arity, j = 0, i + 2
            if j < len(words) and words[j] == '<':
                level, arity = 0, 1
                while j < len(words):
                    level += {'<': 1, '>': -1}.get(words[j], 0)
                    if words[j] == ',' and level == 1:
                        arity += 1
                    if level == 0:
                        break
                    j += 1
            if not stack:
                tops.append((package, nxt, arity))
            header = 'name'
        if header and t in ('extends', 'implements', 'permits'):
            header = 'parents'
        if t == '@' and nxt not in ('interface',):
            annotation, ann_parens = True, 0
        if in_import:
            ctx = 'import'
        elif annotation:
            ctx = 'annotation'
        elif header == 'parents':
            ctx = 'parent'
        elif header == 'name' or (stack and stack[-1] == 'type' and not init):
            ctx = 'signature'
        else:
            ctx = 'other'
        out.append((s, e, ctx, tops[-1] if tops else None))
        # the annotation's end: its name, and its arguments where a `(` follows the name
        if annotation and t != '@':
            if t == '(':
                ann_parens += 1
            elif t == ')':
                ann_parens -= 1
                if ann_parens == 0:
                    annotation = False
            elif ann_parens == 0 and nxt != '.' and t != '.' and nxt != '(':
                annotation = False
        if t == ';':
            in_import = init = False
        if t == '=' and stack and stack[-1] == 'type' and not header:
            init = True
        if t == '{':
            stack.append('type' if header else 'block')
            header = None
        elif t == '}':
            if stack:
                stack.pop()
            init = False
        i += 1
    first = tops[0] if tops else None
    return [(s, e, c, k or first) for s, e, c, k in out]


def listed(text, java_name, scalac_errors):
    """(the listed class, the twin, what it matched) of a row the head builds and scalac refuses, by scalac's errors
    alone; ('', '', '') where it is none."""
    errs = []
    for x in scalac_errors.split(','):
        eid, _, at = x.partition('@')
        parts = at.rsplit(':', 2)
        if len(parts) != 3 or os.path.basename(parts[0]) != java_name:
            return '', '', ''
        errs.append((eid, int(parts[1]), int(parts[2])))
    ps = places(text)
    def at(l, c):
        off = offset_of(text, l, c)
        return next((p for p in ps if p[1] > off), None)
    # every error scalac gives stands in an annotation, a parent, a signature or an import of the Java source
    if any(at(l, c) is None or at(l, c)[2] == 'other' for _, l, c in errs):
        return '', '', ''
    kinds, twin, matched = set(), '', []
    for eid, l, c in errs:
        if eid not in NAMES:
            continue
        p = at(l, c)
        kinds.add('annotation names' if p[2] == 'annotation' else "unreached class's names")
        matched.append(f'{eid} {l}:{c} in {p[2]}')
        if not twin:
            pkg, name, arity = p[3] if p[3] else (package_of(text), '', 0)
            twin = f'{pkg}|{name}|{arity}|{l}:{c + 1}'
    if len(kinds) != 1:
        return '', '', ''
    return kinds.pop(), twin, '; '.join(matched)


def package_of(text):
    m = re.search(r'^\s*package\s+([\w.]+)\s*;', text, re.M)
    return m.group(1) if m else ''


def main():
    args = sys.argv[1:]
    master_teq, head_teq = os.path.abspath(args[0]), os.path.abspath(args[1])
    jobs = int(args[args.index('--jobs') + 1]) if '--jobs' in args else max(1, (os.cpu_count() or 4) // 2)
    files, inputs, broken = corpus.generated(corpus.tag())
    if not files:
        sys.exit(f'judge: no .java file under {corpus.tag()}/tests (set SCALA3_TAG)')
    version = subprocess.run([master_teq, '--version'], capture_output=True, text=True).stdout.strip()
    head_version = subprocess.run([head_teq, '--version'], capture_output=True, text=True).stdout.strip()
    work = tempfile.mkdtemp(prefix='teq-java-judge.')
    try:
        dirs = {}
        for key, name, text in inputs:
            w = corpus.workdir(work, key)
            os.makedirs(w)
            corpus.write(os.path.join(w, name), text)
            corpus.write(os.path.join(w, 'Main.scala'), corpus.MAIN)
            dirs[key] = (w, name)
        keys = [k for k, _, _ in inputs]

        def master(key):
            w, name = dirs[key]
            r = subprocess.run([master_teq, 'compiler', 'check', os.path.join(w, name), os.path.join(w, 'Main.scala'), '--target', 'jvm',
                                '--classpath', SL], capture_output=True, timeout=60)
            return key, 'builds' if r.returncode == 0 else 'refuses'

        def head(key):
            w, name = dirs[key]
            r = subprocess.run([head_teq, 'compiler', 'check', w, '--target', 'jvm', '--classpath', SL, '--threads', '1'],
                               capture_output=True, timeout=60)
            return key, r.returncode == 0
        with concurrent.futures.ThreadPoolExecutor(os.cpu_count() or 4) as ex:
            masters = dict(ex.map(master, keys))
            builds = dict(ex.map(head, keys))

        classes = os.path.join(work, 'judge-classes')
        subprocess.run(['javac', '-nowarn', '-cp', SC, '-d', classes, os.path.join(corpus.HERE, 'judge', 'Judge.java')], check=True)
        chunks = [keys[i::jobs] for i in range(jobs)]

        def scalac(i):
            lst = os.path.join(work, f'list-{i}')
            open(lst, 'w').write(''.join(dirs[k][0] + '\n' for k in chunks[i]))
            r = subprocess.run(['java', '-XX:-UsePerfData', '-Xmx1g', '-Xss512m', '-cp', SC + ':' + classes, 'Judge', SL, lst],
                               capture_output=True, text=True, errors='replace')
            back = {}
            for l in r.stdout.splitlines():
                d, _, v = l.partition('\t')
                if v:
                    back[d] = v
            return {k: back.get(dirs[k][0], 'crashes: no verdict (Judge ended early)') for k in chunks[i]}
        scalacs = {}
        with concurrent.futures.ThreadPoolExecutor(jobs) as ex:
            for part in ex.map(scalac, range(jobs)):
                scalacs.update(part)

        def javac(key):
            w, name = dirs[key]
            r = subprocess.run(['javac', '-J-Xss512m', '-nowarn', '-proc:none', '-d', os.path.join(w, 'javac-out'), os.path.join(w, name)],
                               capture_output=True, timeout=300)
            return 'javac accepts' if r.returncode == 0 else 'javac refuses'
        rows = []
        for key, name, text in inputs:
            verdict, _, errors = scalacs[key].partition('\t')
            tag = twin = matched = ''
            if builds[key] and verdict.startswith('crashes:'):
                tag = f'scalac crashes; {javac(key)}'
            elif builds[key] and verdict.startswith('refuses:'):
                tag, twin, matched = listed(text, name, errors)
                # none of those: the Java source's validity is javac's (docs/COMPATIBILITY.md), a row javac compiles
                # alone listed, scalac reading the source being wrong on it
                if not tag and errors and all(os.path.basename(x.partition('@')[2].rsplit(':', 2)[0]) == name for x in errors.split(',')) and javac(key) == 'javac accepts':
                    tag = 'javac accepts'
            rows.append((key, masters[key], verdict, errors, tag, twin, matched))
    finally:
        shutil.rmtree(work, ignore_errors=True)
    with open(os.path.join(corpus.HERE, 'judged.tsv'), 'w', encoding='utf-8') as fh:
        fh.write(f'# progress.py\'s inputs: key, master\'s verdict with the files named ({version}), scalac 3.8.4\'s, its errors, '
                 f'the listed class, its reached twin and the places it matched (by {head_version}). Written by judge.py.\n')
        for r in rows:
            fh.write('\t'.join(r) + '\n')
    count = lambda pred: sum(1 for r in rows if pred(r))
    print(f'judge: {len(rows)} inputs; master refuses {count(lambda r: r[1] == "refuses")}, scalac accepts '
          f'{count(lambda r: r[2] == "accepts")}, crashes {count(lambda r: r[2].startswith("crashes"))}; listed: annotation names '
          f'{count(lambda r: r[4] == "annotation names")}, unreached class\'s names {count(lambda r: r[4].startswith("unreached"))}, '
          f'scalac crashes {count(lambda r: r[4].startswith("scalac crashes"))}, javac accepts {count(lambda r: r[4] == "javac accepts")}')


if __name__ == '__main__':
    main()
