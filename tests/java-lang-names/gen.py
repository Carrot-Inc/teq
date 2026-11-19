#!/usr/bin/env python3
"""tests/java-lang-names/gen.py <the JDK's java/lang class files> [jobs]: writes tests/java-lang-names/cells.tsv, the
table tests/java-lang-names.sh runs. A cell is one java.lang class written IN FULL in one position of a program: a
val's type, a parameter, a result, a type argument, `_root_.java.lang.X`, `import java.lang.X`, `import java.lang.{X
=> Renamed}`, `import java.lang.*` then the simple name, `classOf`, a type test, a typed pattern, `new` and a parent
where the class has a public or protected constructor of no argument or of one String, a static member (a public
static field, else a public static method of no argument), an annotation's use; the user's own type of the simple name
imported by wildcard beside `import scala.*` and beside `import java.lang.*`, `scala.<Name>` and `Predef.<Name>`; and
`scala.Predef.String`, `Predef.String`, `scala.Predef.Class`, `Predef.Class` and `_root_.scala.Predef.Class`. The
classes: names.txt beside this file (the JDK's public top-level java.lang classes the std defines a class of the name
of somewhere, the builtins String and Object, and Integer to Record of the reference's own lists). Each class's shape
(its type parameters, final, interface, annotation, its constructors and statics) is read from its class file in the
directory given (`jimage extract` of the JDK's lib/modules, java.base/java/lang). Each program is compiled by scalac
3.8.4 and run on the JVM: its output is the cell's expected line; a program scalac refuses is a cell every leg is to
refuse (its expected `REFUSED` and scalac's first error), and one whose run throws is no cell. Columns: name,
position, the program (its lines joined by `\\n`), scalac's output."""
import concurrent.futures, os, re, shutil, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import classfile  # noqa: E402
JL = sys.argv[1]
JOBS = int(sys.argv[2]) if len(sys.argv) > 2 else 6
M2 = os.path.expanduser('~/.cache/coursier/v1/https/repo1.maven.org/maven2/')
SL = M2 + 'org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar'
SC = ':'.join(M2 + j for j in [
    'org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar',
    'org/scala-lang/scala3-compiler_3/3.8.4/scala3-compiler_3-3.8.4.jar',
    'org/scala-lang/scala3-interfaces/3.8.4/scala3-interfaces-3.8.4.jar',
    'org/scala-lang/tasty-core_3/3.8.4/tasty-core_3-3.8.4.jar',
    'org/scala-lang/modules/scala-asm/9.9.0-scala-1/scala-asm-9.9.0-scala-1.jar',
    'org/scala-sbt/compiler-interface/1.10.7/compiler-interface-1.10.7.jar'])
names = [l.strip() for l in open(os.path.join(HERE, 'names.txt')) if l.strip() and not l.startswith('#')]
programs = []
def write(name, pos, body, top=''):
    pkg = f'p_{name}__{pos}'
    programs.append((name, pos, f'package {pkg}\n{top}object Main {{\n  def main(args: Array[String]): Unit = {{\n{body}  }}\n}}\n'))
for name in names:
    c = classfile.read(open(os.path.join(JL, name + '.class'), 'rb').read())
    sig = c['signature'] or ''
    tparams = len(re.findall(r'(?:^<|;)(\w+):', sig)) if sig.startswith('<') else 0
    targs = '[' + ', '.join(['?'] * tparams) + ']' if tparams else ''
    full = f'java.lang.{name}{targs}'
    acc = c['access']
    annotation, interface, final, abstract = acc & 0x2000, acc & 0x0200, acc & 0x0010, acc & 0x0400
    # the user's own type of the simple name, imported by wildcard beside `import scala.*` and beside `import
    # java.lang.*`, and `scala.<Name>` and `Predef.<Name>` selected: what the name reaches when written in full must
    # not shadow or take these
    own = f'package user {{ trait {name} {{ def value: Int }} }}\n'
    use = f'    val c: {name} = new user.{name} {{ def value: Int = 7 }}\n    println(c.value)\n'
    write(name, 'userscala', '    import user.*\n    import scala.*\n' + use, top=own)
    write(name, 'userjavalang', '    import user.*\n    import java.lang.*\n' + use, top=own)
    write(name, 'scalaselect', f'    val v: scala.{name}{targs} = null\n    println(v == null)\n')
    write(name, 'predefselect', f'    val v: Predef.{name}{targs} = null\n    println(v == null)\n')
    if annotation:
        write(name, 'annotation', f'    @java.lang.{name} def f = 1\n    println(f)\n')
        continue
    write(name, 'val', f'    val v: {full} = null\n    println(v == null)\n')
    write(name, 'param', f'    def p(a: {full}): Boolean = a == null\n    println(p(null))\n')
    write(name, 'result', f'    def r(): {full} = null\n    println(r() == null)\n')
    write(name, 'typearg', f'    val l: List[{full}] = List(null)\n    println(l.size)\n')
    write(name, 'root', f'    val v: _root_.java.lang.{name}{targs} = null\n    println(v == null)\n')
    write(name, 'import', f'    import java.lang.{name}\n    val v: {name}{targs} = null\n    println(v == null)\n')
    write(name, 'rename', f'    import java.lang.{{{name} => Renamed}}\n    val v: Renamed{targs} = null\n    println(v == null)\n')
    write(name, 'wildcard', f'    import java.lang.*\n    val v: {name}{targs} = null\n    println(v == null)\n')
    write(name, 'classof', f'    println(classOf[{full}] != null)\n')
    write(name, 'typetest', f'    val a: Any = "s"\n    println(a.isInstanceOf[{full}])\n')
    write(name, 'pattern', f'    val a: Any = "s"\n    println(a match {{ case _: {full} => 1; case _ => 0 }})\n')
    pub_init = {m[2] for m in c['methods'] if m[1] == '<init>' and m[0] & 0x0001}
    prot_init = {m[2] for m in c['methods'] if m[1] == '<init>' and m[0] & 0x0004}
    if not interface and not abstract:
        if '()V' in pub_init:
            write(name, 'new', f'    val v = new java.lang.{name}{targs and "[String]"}()\n    println(v != null)\n')
        elif '(Ljava/lang/String;)V' in pub_init:
            write(name, 'new', f'    val v = new java.lang.{name}("1")\n    println(v != null)\n')
    if not final and not interface and not abstract and (pub_init | prot_init) & {'()V', '(Ljava/lang/String;)V'}:
        arg = '' if '()V' in pub_init | prot_init else '("1")'
        write(name, 'parent', f'    println(new P().isInstanceOf[java.lang.{name}{targs}])\n', top=f'class P extends java.lang.{name}{targs and "[String]"}{arg}\n')
    if interface and not [m for m in c['methods'] if m[0] & 0x0400]:
        write(name, 'parent', f'    println(new P().isInstanceOf[java.lang.{name}{targs}])\n', top=f'class P extends java.lang.{name}{targs and "[String]"}\n')
    statics = [f for f in c['fields'] if f[0] & 0x0009 == 0x0009]
    if statics:
        fld = next((f for f in statics if f[1] in c['constants']), statics[0])[1]
        write(name, 'static', f'    println(java.lang.{name}.{fld}.toString.nonEmpty)\n')
    else:
        meth = next((m for m in c['methods'] if m[0] & 0x0009 == 0x0009 and m[2].startswith('()') and not m[2].endswith(')V')), None)
        if meth:
            write(name, 'static', f'    println(java.lang.{name}.{meth[1]}() != null)\n')
write('Predef', 'String', '    val v: scala.Predef.String = null\n    println(v == null)\n')
write('Predef', 'Class', '    val v: scala.Predef.Class[?] = null\n    println(v == null)\n')
write('Predef', 'bareString', '    val v: Predef.String = null\n    println(v == null)\n')
write('Predef', 'bareClass', '    val v: Predef.Class[?] = null\n    println(v == null)\n')
write('Predef', 'rootClass', '    val v: _root_.scala.Predef.Class[?] = null\n    println(v == null)\n')
def scalac(p):
    name, pos, src = p
    w = tempfile.mkdtemp(prefix='java-lang-names.')
    try:
        f = os.path.join(w, 'Main.scala')
        open(f, 'w').write(src)
        r = subprocess.run(['java', '-XX:-UsePerfData', '-Xmx1g', '-cp', SC, 'dotty.tools.dotc.Main', '-color:never', '-classpath', SL, '-d', w, f], capture_output=True, text=True, timeout=300)
        if r.returncode != 0:
            # a program scalac refuses is a cell whose every leg is to refuse it, with scalac's first error beside
            first = next((l for l in (r.stdout + r.stderr).splitlines() if l.startswith('-- ')), '').split(': ', 1)[0]
            why = next((l.strip(' |') for l in (r.stdout + r.stderr).splitlines() if l.strip().startswith('|') and l.strip(' |^') and not l.strip(' |').startswith(('1 |', '2 |', '3 |', '4 |', '5 |', '6 |', '7 |', '8 |', '9 |'))), '')
            return (name, pos, src, 'REFUSED ' + (first.replace('-- ', '') + ' ' + why).strip()[:120])
        r = subprocess.run(['java', '-XX:-UsePerfData', '-cp', f'{w}:{SL}', f'p_{name}__{pos}.Main'], capture_output=True, text=True, timeout=60)
        return (name, pos, src, r.stdout.strip()) if r.returncode == 0 else None
    finally:
        shutil.rmtree(w, ignore_errors=True)
with concurrent.futures.ThreadPoolExecutor(JOBS) as ex:
    cells = [c for c in ex.map(scalac, programs) if c is not None]
with open(os.path.join(HERE, 'cells.tsv'), 'w') as fh:
    fh.write('# name\tposition\tthe program\tscalac 3.8.4 (its JVM run)  (written by gen.py)\n')
    for name, pos, src, out in cells:
        fh.write('\t'.join([name, pos, src.replace('\\', '\\\\').replace('\n', '\\n'), out.replace('\n', '\\n')]) + '\n')
print(len(programs), 'programs;', len(cells), 'cells')
