#!/usr/bin/env python3
"""patdef_table.py: the generated table of pattern definitions (`val <pattern> = <rhs>`) and its judge against
scalac 3.8.4's own class files (tests/support/patdef-table.sh runs it).

  patdef_table.py gen <dir>
      <dir>/PatDef.scala: a cell a pattern (a single variable, a typed one, a tuple of variables, a tuple with a typed
      element, a tuple with a wildcard, a tuple of wildcards, a tuple over a right side of `Any` and of `Product2`, a
      tuple literal on the right (typed, of two and of 23), a nested tuple, a case
      class's extractor (of two, of 22 and of 23 fields), a cons with `@unchecked`, an extractor object, an alternative
      of literals with `@unchecked`, an interpolator's pattern) in a position (a method's tail block, a block in value position, a class body, a
      lambda's body, a for-comprehension's alias, and once the top of a file: `val (a, b) = f()` in a file of its own,
      <dir>/c_<id>.scala, a member of that file's package object) over an element type (Int, String, a type
      parameter, a value class), where the language takes it. A cell's code is named by its id: the method `c_<id>`,
      in the class position the class `K_<id>`, at the top of a file the package object `c_<id>$package`; main runs
      every cell, which prints its variables, `<id> <values>`. <dir>/cells.txt lists the
      cells, a line each: id, pattern, position, type, and the branch of dotty's `Desugar.makePatDef` it takes.
  patdef_table.py judge <dir>
      <dir>/out/: `scalac.javap` and `teq.javap` (every class of each build, `javap -c -p`), `scalac.out` and `teq.out`
      (the runs), `scalac.javapv` and `teq.javapv` (`javap -v -p`, read for each class's `BootstrapMethods`). Per cell,
      the code of every method whose class or name holds the cell's id and of every lambda body such code makes (an
      `invokedynamic` followed to the method its bootstrap's arguments name, whatever each compiler names it), the
      constant pool's indices left out, a jump's target by its
      place, and the locals by the store whose value each access reads (a store without its slot, a load naming the
      stores that reach it: slots that differ but carry the same values are the same code), compared as a multiset:
      `same`; scalac's once teq's varargs argument (the array made into `ArraySeq$ofRef` and cast to `Seq`) is read as
      scalac's wrap of it (`ScalaRunTime.genericWrapArray`), counted apart as the varargs argument's form alone and
      never among scalac's; or the features of each side (where the failure stands, the test of the scrutinee, a tuple
      made of the variables, the tuple's accessors). Apart from the cell's verdict, the code a general definition past
      22 variables writes for its tuple (from the `TupleXXL` module's load to its last `productElement` read's store,
      the slots the match bound named by the order they are read in) is judged on its own: scalac's, scalac's but for
      the varargs argument's form, that and the boxing convention (`Integer.valueOf` for `BoxesRunTime.boxToInteger`),
      or differing; the cell's verdict stays the whole method's. The values each cell prints must be scalac's.
      Prints a line per cell that differs, the counts by branch and position, and the cells whose values differ; exits
      1 where a value differs, a cell is missing from either side's code or run, a cell is judged twice, or teq's
      build or the reference's panicked (`teq.build`, `ref.build`: the count is the last line's `panics: N`).
  patdef_table.py selftest <work dir>
      The judge over planted bad records (two variables' reads swapped and a jump moved, each judged as differing; a
      value moved, a cell's code missing on one side, a cell missing from a run, a cell printed or listed twice, each
      wrong; a lambda's body changed, and its bootstrap naming another body, each judged as differing; a cell of a
      rung not taken whose code moved from the reference's, wrong; a tuple past 22 made of teq's varargs argument over
      another element, and without its cast, each judged as differing; a tuple past 22's read off by one, without its
      cast, without its unbox, its elements out of order in the array, and its reads cut short, each of their segments
      judged as differing; a panic in teq's build and in the reference's, each wrong and counted) and controls
      (the record as it is, its lambda's body named apart on each side; its slots renumbered alike, each read the same
      store's: the same; a cell of a rung taken moved from the reference's: judged by scalac's alone; a tuple past 22
      made of teq's varargs argument, the cell and its build and reads' segment: the varargs argument's form alone).
"""
import collections, os, re, shutil, sys

TYPES = [
    # (name, the element type, three values, the type parameters of a cell's method)
    ('int', 'Int', ('1', '2', '3'), ''),
    ('str', 'String', ('"a"', '"b"', '"c"'), ''),
    ('gen', 'A', ('"g"', '"h"', '"i"'), '[A]'),
    ('vc', 'V', ('new V(1)', 'new V(2)', 'new V(3)'), ''),
]

# (name, the definition with {T} the element type, the variables it binds, the parameters it reads, the branch)
PATTERNS = [
    ('single', 'val x = p0', ['x'], 'p0', 'IdPattern'),
    ('typed', 'val x: {T} = p0', ['x'], 'p0', 'IdPattern'),
    ('tup', 'val (a, b) = p', ['a', 'b'], 'p', 'MatchableTuple'),
    ('tuptyped', 'val (a: {T}, b) = p', ['a', 'b'], 'p', 'MatchableTuple'),
    ('tupwild', 'val (a, _) = p', ['a'], 'p', 'MatchableTuple'),
    ('tupany', 'val (a, b) = x', ['a', 'b'], 'x', 'MatchableTuple'),
    ('tupprod', 'val (a, b) = pr', ['a', 'b'], 'pr', 'MatchableTuple'),
    ('tupnone', 'val (_, _) = p', [], 'p', 'MatchableTuple'),
    ('simple', 'val (a, b) = (p0, p1)', ['a', 'b'], 'p0 p1', 'SimpleTuple'),
    ('simpletyped', 'val (a, b): ({T}, {T}) = (p0, p1)', ['a', 'b'], 'p0 p1', 'SimpleTuple'),
    ('simple23typed', 'val (a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23): ({T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}, {T}) = (p0, p0, p0, p0, p0, p0, p0, p0, p0, p0, p0, p0, p0, p0, p0, p0, p0, p0, p0, p0, p0, p0, p1)', ['a1', 'a23'], 'p0 p1', 'SimpleTuple'),
    ('nested', 'val ((a, b), c) = q', ['a', 'b', 'c'], 'q', 'None'),
    ('cc', 'val Pt(a, b) = pt', ['a', 'b'], 'pt', 'None'),
    ('cons', 'val h :: _ = xs: @unchecked', ['h'], 'xs', 'None'),
    ('extr', 'val Ex(a, b) = bx', ['a', 'b'], 'bx', 'None'),
    ('alt', 'val (1 | 2) = p0: @unchecked', [], 'p0', 'None'),
    ('interp', 'val s"$a-$b" = str', ['a', 'b'], 'str', 'None'),
    ('tupxxl', 'val (a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23) = w', ['a1', 'a23'], 'w', 'MatchableTuple'),
    ('cc22', 'val P22(a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22) = pt22', ['a1', 'a22'], 'pt22', 'None'),
    ('cc23', 'val P23(a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23) = pt23', ['a1', 'a23'], 'pt23', 'None'),
]
POSITIONS = ['tail', 'value', 'class', 'lambda', 'foralias', 'toplevel']


def legal(pat, pos, ty):
    # A file's top: one cell, a tuple of two Ints from a call (`val (a, b) = f()`, a package object's member).
    if pos == 'toplevel':
        return pat == 'tup' and ty == 'int'
    if pat == 'alt' and ty != 'int':
        return False
    if pat == 'interp' and ty != 'str':
        return False
    # A type test on a type parameter is unchecked: scalac warns, and the cell would say nothing more.
    if pat == 'tuptyped' and ty == 'gen':
        return False
    # A for's alias takes the patterns a generator's body could: a refutable one only with its check.
    if pos == 'foralias' and pat in ('cons', 'alt', 'typed', 'single', 'simpletyped', 'simple23typed', 'tupany', 'tupprod'):
        return False
    return True


def params(reads, T):
    out = []
    for r in reads.split():
        out.append({'p0': f'p0: {T}', 'p1': f'p1: {T}', 'p': f'p: ({T}, {T})', 'q': f'q: (({T}, {T}), {T})',
                    'pt': f'pt: Pt[{T}]', 'xs': f'xs: List[{T}]', 'bx': f'bx: Box[{T}]', 'str': 'str: String',
                    'w': 'w: (' + ', '.join([T] * 23) + ')', 'pt22': f'pt22: P22[{T}]', 'pt23': f'pt23: P23[{T}]',
                    'x': 'x: Any', 'pr': f'pr: Product2[{T}, {T}]'}[r])
    return ', '.join(out)


def args(reads, vals):
    a, b, c = vals
    out = []
    for r in reads.split():
        out.append({'p0': a if r == 'p0' else b, 'p1': b, 'p': f'({a}, {b})', 'q': f'(({a}, {b}), {c})',
                    'pt': f'Pt({a}, {b})', 'xs': f'List({a}, {b})', 'bx': f'new Box({a}, {b})', 'str': '"x-y"',
                    'w': '(' + ', '.join([a] * 22 + [c]) + ')', 'pt22': 'P22(' + ', '.join([a] * 21 + [c]) + ')',
                    'pt23': 'P23(' + ', '.join([a] * 22 + [c]) + ')', 'x': f'({a}, {b})', 'pr': f'({a}, {b})'}[r])
    return ', '.join(out)


def cells():
    for pname, pdef, vars_, reads, branch in PATTERNS:
        for pos in POSITIONS:
            for tname, T, vals, tps in TYPES:
                if legal(pname, pos, tname):
                    yield (f'{pname}_{pos}_{tname}', pname, pos, tname, branch, pdef.replace('{T}', T), vars_, reads, T, vals, tps)


def gen(d):
    os.makedirs(d, exist_ok=True)
    lines = ['final class V(val u: Int) extends AnyVal { override def toString = s"V($u)" }',
             'case class Pt[A](x: A, y: A)',
             'case class P22[A](a1: A, a2: A, a3: A, a4: A, a5: A, a6: A, a7: A, a8: A, a9: A, a10: A, a11: A, a12: A, a13: A, a14: A, a15: A, a16: A, a17: A, a18: A, a19: A, a20: A, a21: A, a22: A)',
             'case class P23[A](a1: A, a2: A, a3: A, a4: A, a5: A, a6: A, a7: A, a8: A, a9: A, a10: A, a11: A, a12: A, a13: A, a14: A, a15: A, a16: A, a17: A, a18: A, a19: A, a20: A, a21: A, a22: A, a23: A)',
             'final class Box[A](val l: A, val r: A)',
             'object Ex { def unapply[A](b: Box[A]): Some[(A, A)] = Some((b.l, b.r)) }',
             'object S { val log = new StringBuilder; def out(x: Any): Unit = { log.append(\' \').append(x); () }; def out(x: Int): Unit = { log.append(\' \').append(x); () } }',
             '']
    body, calls, listing = [], [], []
    for cid, pname, pos, tname, branch, pdef, vars_, reads, T, vals, tps in cells():
        ps = params(reads, T)
        uses = '; '.join(f'S.out({v})' for v in vars_) or '()'
        if pos == 'tail':
            body.append(f'  def c_{cid}{tps}({ps}): Unit = {{ {pdef}; {uses} }}')
        elif pos == 'value':
            body.append(f'  def c_{cid}{tps}({ps}): Unit = {{ val r: Int = {{ {pdef}; {uses}; 0 }}; S.out(r) }}')
        elif pos == 'class':
            lines.append(f'class K_{cid}{tps}({ps}) {{ {pdef}; def run(): Unit = {{ {uses} }} }}')
            names = ', '.join(r for r in reads.split())
            body.append(f'  def c_{cid}{tps}({ps}): Unit = new K_{cid}({names}).run()')
        elif pos == 'lambda':
            body.append(f'  def c_{cid}{tps}({ps}): Unit = {{ val f = (u: Unit) => {{ {pdef}; {uses}; "" }}; S.out(f(())) }}')
        elif pos == 'foralias':
            alias = pdef[len('val '):]
            body.append(f'  def c_{cid}{tps}({ps}): Unit = for {{ u <- List(()); {alias} }} {{ {uses} }}')
        elif pos == 'toplevel':
            a, b, _ = vals
            open(os.path.join(d, f'c_{cid}.scala'), 'w').write(
                f'def f_{cid}(): ({T}, {T}) = ({a}, {b})\nval (a_{cid}, b_{cid}) = f_{cid}()\n')
            body.append(f'  def c_{cid}(): Unit = {{ S.out(a_{cid}); S.out(b_{cid}) }}')
            calls.append(f'    S.log.setLength(0); c_{cid}(); println("{cid}" + S.log)')
            listing.append(f'{cid} {pname} {pos} {tname} {branch}')
            continue
        call_args = args(reads, vals)
        calls.append(f'    S.log.setLength(0); c_{cid}({call_args}); println("{cid}" + S.log)')
        listing.append(f'{cid} {pname} {pos} {tname} {branch}')
    lines.append('object PatDef {')
    lines += body
    lines.append('  def main(args: Array[String]): Unit = {')
    lines += calls
    lines += ['  }', '}', '']
    open(os.path.join(d, 'PatDef.scala'), 'w').write('\n'.join(lines))
    open(os.path.join(d, 'cells.txt'), 'w').write('\n'.join(listing) + '\n')
    print(f'patdef table: {len(listing)} cells')
    return 0


LOAD = re.compile(r'^([ilfda])load(?:_(\d))?$')
STORE = re.compile(r'^([ilfda])store(?:_(\d))?$')
ENDS = ('goto', 'goto_w', 'return', 'ireturn', 'lreturn', 'freturn', 'dreturn', 'areturn', 'athrow', 'tableswitch', 'lookupswitch')


def canonical(code, handlers):
    """A method's code with its locals named by the store whose value each access reads (slots that differ but
    carry the same values are the same code): a store loses its slot; a load (and `iinc`) names the stores that reach it, by their place in the code,
    or the parameter it reads; a jump names its target by its place. `code`: [(pc, op, rest)]; `handlers`: [(from,
    to, target)] pcs."""
    at = {pc: k for k, (pc, _, _) in enumerate(code)}
    def slot_of(op, rest):
        m = LOAD.match(op) or STORE.match(op)
        if m:
            return int(m.group(2)) if m.group(2) is not None else int(rest.split()[0])
        if op == 'iinc':
            return int(rest.split(',')[0])
        return None
    succ = []
    for k, (pc, op, rest) in enumerate(code):
        out = []
        if op.startswith('if') or op in ('goto', 'goto_w'):
            t = rest.split()[0]
            if t.isdigit() and int(t) in at:
                out.append(at[int(t)])
        if op in ('tableswitch', 'lookupswitch'):
            out += [at[int(t)] for t in re.findall(r'(\d+)', rest) if int(t) in at]
        if op not in ENDS and k + 1 < len(code):
            out.append(k + 1)
        for f, t, h in handlers:
            if f <= pc < t and h in at:
                out.append(at[h])
        succ.append(out)
    # Reaching stores per slot: a set of definitions (an instruction's place, or `p<slot>` before any).
    ins = [None] * len(code)
    ins[0] = {}
    work = [0] if code else []
    while work:
        k = work.pop()
        state = dict(ins[k])
        pc, op, rest = code[k]
        s = slot_of(op, rest)
        if s is not None and (STORE.match(op) or op == 'iinc'):
            state[s] = frozenset([k])
        for n in succ[k]:
            old = ins[n]
            if old is None:
                ins[n] = dict(state)
                work.append(n)
                continue
            changed = False
            merged = dict(old)
            # A slot no path has stored to reads the parameter it holds.
            for sl in set(old) | set(state):
                v = old.get(sl, frozenset(['p%d' % sl])) | state.get(sl, frozenset(['p%d' % sl]))
                if v != old.get(sl):
                    merged[sl] = v
                    changed = True
            if changed:
                ins[n] = merged
                work.append(n)
    out = []
    for k, (pc, op, rest) in enumerate(code):
        state = ins[k] or {}
        s = slot_of(op, rest)
        m = LOAD.match(op)
        if m:
            defs = sorted(str(d) for d in state.get(s, frozenset(['p%d' % s])))
            out.append(f'{m.group(1)}load {",".join(defs)}')
        elif STORE.match(op):
            out.append(f'{STORE.match(op).group(1)}store')
        elif op == 'iinc':
            defs = sorted(str(d) for d in state.get(s, frozenset(['p%d' % s])))
            out.append(f'iinc {",".join(defs)} by {rest.split(",")[1].strip()}')
        elif op.startswith('if') or op in ('goto', 'goto_w'):
            t = rest.split()[0]
            out.append(f'{op} @{at.get(int(t), "?") if t.isdigit() else t}')
        else:
            r = re.sub(r'#\d+(, *\d+)?\s*', '', rest).strip()
            out.append((op + ' ' + re.sub(r'^// ', '', r)).strip())
    return out


def methods(path, rewrite=None, raw_too=False):
    """{(class, method declaration): canonical code} of a `javap -c -p` text (`canonical`), and {(class, method
    declaration): [the bootstrap index of each `invokedynamic` in its code]}. `rewrite`, given, takes each method's
    code ([(pc, op, rest)]) to another before it is made canonical."""
    out, indy, cls, cur, section = {}, {}, None, None, None
    raw = {}
    for line in open(path, errors='replace'):
        line = line.rstrip()
        m = re.match(r'^(?:[a-z ]+ )?(?:class|interface) (\S+)', line)
        if m:
            cls = m.group(1)
            continue
        bare = line.replace('<init>', '\0init\0').replace('<clinit>', '\0clinit\0')
        while re.search(r'<[^<>]*>', bare):
            bare = re.sub(r'<[^<>]*>', '', bare)
        bare = bare.replace('\0init\0', '<init>').replace('\0clinit\0', '<clinit>')
        m = re.match(r'^  (?:[a-z ]+ )?(?:\S+ )?([\w$<>]+)\((.*)\);$', bare)
        if m:
            cur = raw.setdefault((cls, f'{m.group(1)}({m.group(2)})'), {'code': [], 'exc': [], 'switch': None, 'indy': []})
            section = 'code'
            continue
        if cur is None:
            continue
        st = line.strip()
        if st == 'Exception table:':
            section = 'exc'
            continue
        if section == 'exc':
            mm = re.match(r'(\d+)\s+(\d+)\s+(\d+)\s+', st)
            if mm:
                cur['exc'].append(tuple(int(x) for x in mm.groups()))
                continue
            if not st.startswith('from'):
                section = None
        if section == 'code':
            if cur['switch'] is not None:
                if st == '}':
                    pc, op, rest = cur['code'][cur['switch']]
                    cur['code'][cur['switch']] = (pc, op, rest)
                    cur['switch'] = None
                else:
                    mm = re.match(r'(?:-?\d+|default): (\d+)', st)
                    if mm:
                        pc, op, rest = cur['code'][cur['switch']]
                        cur['code'][cur['switch']] = (pc, op, rest + ' ' + mm.group(1))
                continue
            mm = re.match(r'(\d+): (\w+)(.*)', st)
            if mm:
                cur['code'].append((int(mm.group(1)), mm.group(2), mm.group(3).strip()))
                bm = re.search(r'InvokeDynamic #(\d+):', mm.group(3)) if mm.group(2) == 'invokedynamic' else None
                if bm:
                    cur['indy'].append(int(bm.group(1)))
                if mm.group(2) in ('tableswitch', 'lookupswitch'):
                    cur['switch'] = len(cur['code']) - 1
    for key, m in raw.items():
        out[key] = canonical(rewrite(m['code']) if rewrite else m['code'], m['exc'])
        indy[key] = m['indy']
    if raw_too:
        return out, indy, {key: m['code'] for key, m in raw.items()}
    return out, indy


ARRAY_SEQ = 'class scala/collection/immutable/ArraySeq$ofRef'
ARRAY_SEQ_INIT = 'Method scala/collection/immutable/ArraySeq$ofRef."<init>":([Ljava/lang/Object;)V'
RUNTIME_MODULE = '#0 // Field scala/runtime/ScalaRunTime$.MODULE$:Lscala/runtime/ScalaRunTime$;'
GENERIC_WRAP = '#0 // Method scala/runtime/ScalaRunTime$.genericWrapArray:(Ljava/lang/Object;)Lscala/collection/immutable/ArraySeq;'
SEQ_CAST = 'class scala/collection/immutable/Seq'


def varargs_as_scalac(code):
    """teq's varargs argument, the array of references made into `ArraySeq$ofRef` (`new`, `dup`, the array, its
    `<init>`, the cast to `Seq` the argument's parameter takes), written as scalac's for a varargs parameter of
    `Any`: the `ScalaRunTime` module, the array, `genericWrapArray`. Only where each `new` of the class has its `dup`
    after it, each constructor its cast, and the constructors are as many."""
    news = [k for k, (_, op, rest) in enumerate(code) if op == 'new' and rest.endswith(ARRAY_SEQ)]
    inits = [k for k, (_, op, rest) in enumerate(code) if op == 'invokespecial' and rest.endswith(ARRAY_SEQ_INIT)]
    if not news or len(news) != len(inits) or any(k + 1 >= len(code) or code[k + 1][1] != 'dup' for k in news):
        return code
    if any(k + 1 >= len(code) or code[k + 1][1] != 'checkcast' or not code[k + 1][2].endswith(SEQ_CAST) for k in inits):
        return code
    dropped = {k + 1 for k in news} | {k + 1 for k in inits}
    out = []
    for k, (pc, op, rest) in enumerate(code):
        if k in dropped:
            continue
        if k in news:
            out.append((pc, 'getstatic', RUNTIME_MODULE))
        elif k in inits:
            out.append((pc, 'invokevirtual', GENERIC_WRAP))
        else:
            out.append((pc, op, rest))
    return out


BOXES = {f'Method java/lang/{b}.valueOf:({c})Ljava/lang/{b};': f'Method scala/runtime/BoxesRunTime.boxTo{n}:({c})Ljava/lang/{b};'
         for b, c, n in (('Integer', 'I', 'Integer'), ('Long', 'J', 'Long'), ('Double', 'D', 'Double'), ('Float', 'F', 'Float'),
                         ('Short', 'S', 'Short'), ('Byte', 'B', 'Byte'), ('Character', 'C', 'Character'), ('Boolean', 'Z', 'Boolean'))}


def boxes_as_scalac(code):
    """teq's boxing of a primitive (`Integer.valueOf`) written as scalac's (`BoxesRunTime.boxToInteger`), the boxing
    convention's form."""
    out = []
    for pc, op, rest in code:
        for mine, theirs in BOXES.items():
            if op == 'invokestatic' and rest.endswith(mine):
                rest = '#0 // ' + theirs
        out.append((pc, op, rest))
    return out


def xxl_segment(code):
    """The code a general pattern definition past 22 variables writes for its tuple: from the `TupleXXL` module's load
    its first `apply` is called on to the store of the variable its last `productElement` read gives; None without
    both (a tuple pattern past 22 over a `TupleXXL`, its module's `unapplySeq` alone, is none)."""
    applies = [k for k, (_, op, rest) in enumerate(code) if op == 'invokevirtual' and 'Method scala/runtime/TupleXXL$.apply:(Lscala/collection/immutable/Seq;)' in rest]
    if not applies:
        return None
    start = next((k for k in range(applies[0], -1, -1) if code[k][1] == 'getstatic' and code[k][2].endswith('Field scala/runtime/TupleXXL$.MODULE$:Lscala/runtime/TupleXXL$;')), None)
    reads = [k for k, (_, op, rest) in enumerate(code) if op == 'invokevirtual' and 'Method scala/runtime/TupleXXL.productElement:(I)' in rest]
    if start is None or not reads or reads[-1] < start:
        return None
    end = reads[-1] + 1
    while end < len(code) and not STORE.match(code[end][1]):
        end += 1
    return code[start:end + 1]


def segment_canonical(seg):
    """A segment's code as `canonical` gives it, a slot the segment reads before it stores it named by the order it is
    first read in (the variables the match bound, whatever their slots)."""
    names = {}
    def outer(m):
        return 'o%d' % names.setdefault(m.group(1), len(names))
    return [re.sub(r'\bp(\d+)\b', outer, i) for i in canonical(seg, [])]


def xxl_verdict(sc, tc):
    """The segment of teq's (`tc`) against scalac's (`sc`): `same`, `varargs` (scalac's once teq's varargs argument is
    read as scalac's), `boxing` (once its boxing is too), else `differs`."""
    if tc is None:
        return 'differs'
    want = segment_canonical(sc)
    if segment_canonical(tc) == want:
        return 'same'
    if segment_canonical(varargs_as_scalac(tc)) == want:
        return 'varargs'
    if segment_canonical(boxes_as_scalac(varargs_as_scalac(tc))) == want:
        return 'boxing'
    return 'differs'


def bootstraps(path):
    """{class: {bootstrap index: (class, method name) of the body its arguments name}} of a `javap -v -p` text: the
    first method handle among a bootstrap method's arguments (`LambdaMetafactory`'s implementation)."""
    out, cls, k, inside = collections.defaultdict(dict), None, None, False
    if not os.path.exists(path):
        return out
    for line in open(path, errors='replace'):
        line = line.rstrip()
        m = re.match(r'^(?:[a-z ]+ )?(?:class|interface) (\S+)', line)
        if m:
            cls, inside = re.sub(r'<.*', '', m.group(1)), False
            continue
        if line == 'BootstrapMethods:':
            inside = True
            continue
        if inside and line and not line.startswith(' '):
            inside = False
        if not inside:
            continue
        mm = re.match(r'^  (\d+): ', line)
        if mm:
            k = int(mm.group(1))
            continue
        mm = re.match(r'^\s+#\d+ REF_\w+ ([\w/$]+)\.([\w$<>]+):', line)
        if mm and k is not None and k not in out[cls]:
            out[cls][k] = (mm.group(1).replace('/', '.'), mm.group(2))
    return out


def of_cell(ms, cid, indy=None, bsm=None):
    """The cell's methods' codes, sorted: those of a class named for it, those whose name holds its id, and the
    lambda bodies their `invokedynamic`s make, followed through `bsm` (`bootstraps`)."""
    return sorted(tuple(ms[k]) for k in cell_keys(ms, cid, indy, bsm))


def cell_keys(ms, cid, indy=None, bsm=None):
    """The keys of `of_cell`'s methods."""
    key = re.compile(r'(?:^|[^A-Za-z0-9])(?:c|K)_' + re.escape(cid) + r'(?:[^A-Za-z0-9_]|$)')
    keys = [k for k in ms if key.search(k[0] or '') or key.search(k[1])]
    taken, todo = set(keys), list(keys)
    bare = lambda c: re.sub(r'<.*', '', c or '')
    while todo and indy is not None and bsm is not None:
        cls, decl = todo.pop()
        for b in indy.get((cls, decl), []):
            target = bsm.get(bare(cls), {}).get(b)
            if target is None:
                continue
            for k in ms:
                if bare(k[0]) == target[0] and k[1].startswith(target[1] + '(') and k not in taken:
                    taken.add(k)
                    todo.append(k)
    return taken


def features(codes):
    """Where the failure stands (before or after the first test), the scrutinee's test, a tuple made, the accessors."""
    flat = [i for c in codes for i in c]
    first_test = next((k for k, i in enumerate(flat) if i.split()[0] in ('ifnull', 'ifnonnull', 'instanceof', 'ifeq', 'ifne')), None)
    error = next((k for k, i in enumerate(flat) if 'scala/MatchError' in i and i.startswith('new')), None)
    where = 'none' if error is None else ('first' if first_test is None or error < first_test else 'after')
    tests = sorted({i.split()[0] + (' ' + i.split()[-1].split('/')[-1] if i.startswith('instanceof') else '') for i in flat if i.split()[0] in ('ifnull', 'ifnonnull', 'instanceof')})
    made = sorted({re.sub(r'.*class (scala/Tuple\w*).*', r'\1', i) for i in flat if i.startswith('new') and 'scala/Tuple' in i})
    acc = sorted({re.sub(r'.*\.(_\d+(?:\$mc\w+\$sp)?):.*', r'\1', i) for i in flat if re.search(r'Tuple\w*\._\d', i)})
    return f'failure {where}; tests {",".join(tests) or "-"}; tuple {",".join(made) or "-"}; accessors {",".join(acc) or "-"}'


def runs(path):
    out = collections.defaultdict(list)
    if os.path.exists(path):
        for line in open(path, errors='replace'):
            parts = line.rstrip('\n').split(' ', 1)
            if parts and parts[0]:
                out[parts[0]].append(parts[1] if len(parts) > 1 else '')
    return out


def taken(pname, branch, pos):
    """Whether the lowering writes the cell's definition in dotty's form: `makePatDef`'s `MatchableTuple` and
    `None` branches and a typed `SimpleTuple` among a block's statements (a method's tail, a value, a lambda's body),
    and a for-comprehension's alias whatever its pattern; a class body's (the parser's), an `IdPattern` and an untyped
    `SimpleTuple` (the parser's and the typer's split) are not."""
    if pos == 'foralias':
        return True
    return (branch in ('MatchableTuple', 'None') or pname in ('simpletyped', 'simple23typed')) and pos in ('tail', 'value', 'lambda')


def judge(d):
    cells_ = [l.split() for l in open(os.path.join(d, 'cells.txt')) if l.strip()]
    o = os.path.join(d, 'out')
    (sm, si, sraw), (tm, ti, traw) = methods(os.path.join(o, 'scalac.javap'), raw_too=True), methods(os.path.join(o, 'teq.javap'), raw_too=True)
    tv, _ = methods(os.path.join(o, 'teq.javap'), varargs_as_scalac)
    sb, tb = bootstraps(os.path.join(o, 'scalac.javapv')), bootstraps(os.path.join(o, 'teq.javapv'))
    sr, tr = runs(os.path.join(o, 'scalac.out')), runs(os.path.join(o, 'teq.out'))
    # The reference's code, where one was built: a cell of a rung the head does not take keeps it.
    rm = methods(os.path.join(o, 'ref.javap')) if os.path.exists(os.path.join(o, 'ref.javap')) else None
    rb = bootstraps(os.path.join(o, 'ref.javapv'))
    wrong, same, varargs, by = [], 0, 0, collections.Counter()
    segments = collections.Counter()
    kept = 0
    lines = []
    seen = set()
    panics = 0
    for side in ('teq', 'ref'):
        build = os.path.join(o, f'{side}.build')
        n = sum(1 for line in open(build, errors='replace') if ' panicked at ' in line) if os.path.exists(build) else 0
        if n:
            wrong.append(f'{side}\'s build panicked {n} time(s)')
        panics += n
    for cid, pname, pos, tname, branch in cells_:
        if cid in seen:
            wrong.append(f'{cid}: listed twice')
            continue
        seen.add(cid)
        sc, tc = of_cell(sm, cid, si, sb), of_cell(tm, cid, ti, tb)
        if not sc or not tc:
            wrong.append(f'{cid}: no code on the {"scalac" if not sc else "teq"} side')
            continue
        if len(sr.get(cid, [])) != 1 or len(tr.get(cid, [])) != 1:
            wrong.append(f'{cid}: printed {len(sr.get(cid, []))} times by scalac\'s run, {len(tr.get(cid, []))} by teq\'s')
            continue
        if sr[cid] != tr[cid]:
            wrong.append(f'{cid}: values {tr[cid][0]!r} where scalac prints {sr[cid][0]!r}')
        if rm is not None and not taken(pname, branch, pos):
            if of_cell(rm[0], cid, rm[1], rb) == tc:
                kept += 1
            else:
                wrong.append(f'{cid}: its code moved from the reference\'s, its rung ({branch}, {pos}) not taken')
        ss = [g for g in (xxl_segment(sraw[k]) for k in sorted(cell_keys(sm, cid, si, sb))) if g]
        if ss:
            ts = [g for g in (xxl_segment(traw[k]) for k in sorted(cell_keys(tm, cid, ti, tb))) if g]
            verdict = xxl_verdict(ss[0], ts[0] if len(ts) == 1 and len(ss) == 1 else None)
            segments[verdict] += 1
            lines.append(f'tuple past 22 {cid}: {verdict}')
        if sc == tc:
            same += 1
            by[(branch, pos, 'same')] += 1
        elif sc == of_cell(tv, cid, ti, tb):
            varargs += 1
            by[(branch, pos, 'varargs')] += 1
            lines.append(f'varargs {cid} ({branch}): scalac\'s code but for the varargs argument\'s form')
        else:
            by[(branch, pos, 'differs')] += 1
            lines.append(f'differs {cid} ({branch}): scalac {features(sc)} | teq {features(tc)}')
    for l in lines:
        print(l)
    print('by branch and position (same / the varargs argument\'s form alone / differs):')
    for branch in sorted({b for b, _, _ in by}):
        row = ', '.join(f'{pos} {by[(branch, pos, "same")]}/{by[(branch, pos, "varargs")]}/{by[(branch, pos, "differs")]}' for pos in POSITIONS if by[(branch, pos, 'same')] or by[(branch, pos, 'varargs')] or by[(branch, pos, 'differs')])
        print(f'  {branch}: {row}')
    print(f'the tuple past 22\'s build and reads: {sum(segments.values())} cells; scalac\'s in {segments["same"]}, the varargs argument\'s form alone in {segments["varargs"]}, with the boxing convention too in {segments["boxing"]}, differing in {segments["differs"]}')
    if rm is not None:
        print(f'rungs not taken: {kept} of {sum(1 for c in cells_ if not taken(c[1], c[4], c[2]))} cells with the reference\'s code')
    print(f'patdef table: {len(cells_)} cells; teq\'s code scalac\'s in {same}, the varargs argument\'s form alone in {varargs}, differing in {len(cells_) - same - varargs - len([w for w in wrong if "no code" in w or "printed" in w or "twice" in w or "moved" in w])}; wrong {len(wrong)}; panics: {panics}')
    for w in wrong[:40]:
        print(f'  WRONG {w}')
    return 1 if wrong else 0


def selftest(work):
    shutil.rmtree(work, ignore_errors=True)
    os.makedirs(work)
    results = []
    tup = ('  public void c_tup_tail_int(scala.Tuple2);\n    Code:\n       0: aload_1\n       1: astore_2\n       2: aload_2\n'
           '       3: ifnull        16\n       6: iconst_1\n       7: istore_3\n       8: iconst_2\n       9: istore        4\n'
           '      11: iload_3\n      12: iload         4\n      14: isub\n      15: pop\n      16: return\n')
    single = '  public void c_single_tail_int(int);\n    Code:\n       0: return\n'
    # A lambda cell: its body named apart on each side, reached through the bootstrap method alone.
    lam = ('  public void c_tup_lambda_int(scala.Tuple2);\n    Code:\n       0: invokedynamic #5,  0             '
           '// InvokeDynamic #0:apply:()Lscala/Function0;\n       5: pop\n       6: return\n')
    body = '  private static final void {}(int);\n    Code:\n       0: iload_0\n       1: iconst_3\n       2: iadd\n       3: pop\n       4: return\n'
    names = {'scalac': '$anonfun$1', 'teq': '$anonfun$7'}
    bsm = ('class PatDef$\nBootstrapMethods:\n  0: #9 REF_invokeStatic java/lang/invoke/LambdaMetafactory.metafactory:()V\n'
           '    Method arguments:\n      #2 ()V\n      #3 REF_invokeStatic PatDef$.{}:(I)V\n      #2 ()V\n')

    def record(label, want_ok, want_same, plant, want_varargs=None, want_segments=None, want_panics=None):
        d = os.path.join(work, re.sub(r'[^A-Za-z0-9]+', '-', label))
        os.makedirs(os.path.join(d, 'out'))
        open(os.path.join(d, 'cells.txt'), 'w').write('tup_tail_int tup tail int MatchableTuple\nsingle_tail_int single tail int IdPattern\n'
                                                       'tup_lambda_int tup lambda int MatchableTuple\n')
        for side in ('scalac', 'teq', 'ref'):
            open(os.path.join(d, 'out', f'{side}.javap'), 'w').write('class PatDef$\n' + tup + single + lam + body.format(names.get(side, '$anonfun$1')))
            open(os.path.join(d, 'out', f'{side}.javapv'), 'w').write(bsm.format(names.get(side, '$anonfun$1')))
            open(os.path.join(d, 'out', f'{side}.out'), 'w').write('tup_tail_int 1 2\nsingle_tail_int 1\ntup_lambda_int 3\n')
        plant(d)
        import io, contextlib
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            code = judge(d)
        text = buf.getvalue()
        same = re.search(r"teq's code scalac's in (\d+)", text)
        varargs = re.search(r"teq's code scalac's in \d+, the varargs argument's form alone in (\d+)", text)
        right = (code == 0) == want_ok and (want_ok or 'WRONG' in text) and (want_same is None or (same and int(same.group(1)) == want_same))
        right = right and (want_varargs is None or (varargs and int(varargs.group(1)) == want_varargs))
        seg = re.search(r"build and reads: (\d+) cells; scalac's in (\d+), the varargs argument's form alone in (\d+), with the boxing convention too in (\d+), differing in (\d+)", text)
        right = right and (want_segments is None or (seg and tuple(int(x) for x in seg.groups()) == want_segments))
        right = right and (want_panics is None or text.rstrip().split('\n')[-1].endswith(f'panics: {want_panics}') or f'; panics: {want_panics}\n' in text)
        results.append((label, right, text.strip().split('\n')[-1]))

    xxl = {'scalac': ('       0: getstatic     #2                  // Field scala/runtime/TupleXXL$.MODULE$:Lscala/runtime/TupleXXL$;\n'
                      '       3: getstatic     #3                  // Field scala/runtime/ScalaRunTime$.MODULE$:Lscala/runtime/ScalaRunTime$;\n'
                      '       6: iconst_1\n       7: anewarray     #4                  // class java/lang/Object\n'
                      '      10: dup\n      11: iconst_0\n      12: aload_1\n      13: aastore\n'
                      '      14: invokevirtual #5                  // Method scala/runtime/ScalaRunTime$.genericWrapArray:(Ljava/lang/Object;)Lscala/collection/immutable/ArraySeq;\n'
                      '      17: invokevirtual #6                  // Method scala/runtime/TupleXXL$.apply:(Lscala/collection/immutable/Seq;)Lscala/runtime/TupleXXL;\n'
                      '      20: pop\n      21: return\n'),
           'teq': ('       0: getstatic     #2                  // Field scala/runtime/TupleXXL$.MODULE$:Lscala/runtime/TupleXXL$;\n'
                   '       3: new           #7                  // class scala/collection/immutable/ArraySeq$ofRef\n       6: dup\n'
                   '       7: iconst_1\n       8: anewarray     #4                  // class java/lang/Object\n'
                   '      11: dup\n      12: iconst_0\n      13: aload_1\n      14: aastore\n'
                   '      15: invokespecial #8                  // Method scala/collection/immutable/ArraySeq$ofRef."<init>":([Ljava/lang/Object;)V\n'
                   '      18: checkcast     #9                  // class scala/collection/immutable/Seq\n'
                   '      21: invokevirtual #6                  // Method scala/runtime/TupleXXL$.apply:(Lscala/collection/immutable/Seq;)Lscala/runtime/TupleXXL;\n'
                   '      24: pop\n      25: return\n')}

    def made_xxl(teq_edit=None):
        def plant(d):
            for side in ('scalac', 'teq', 'ref'):
                p = os.path.join(d, 'out', f'{side}.javap')
                text = open(p).read()
                code = xxl['scalac' if side == 'scalac' else 'teq']
                if side == 'teq' and teq_edit:
                    code = code.replace(*teq_edit)
                start = text.index('  public void c_tup_tail_int(scala.Tuple2);\n    Code:\n') + len('  public void c_tup_tail_int(scala.Tuple2);\n    Code:\n')
                end = text.index('  public void c_single_tail_int')
                open(p, 'w').write(text[:start] + code + text[end:])
        return plant

    seg_scalac = [
        '       0: getstatic     #2                  // Field scala/runtime/TupleXXL$.MODULE$:Lscala/runtime/TupleXXL$;',
        '       3: getstatic     #3                  // Field scala/runtime/ScalaRunTime$.MODULE$:Lscala/runtime/ScalaRunTime$;',
        '       6: iconst_2', '       7: anewarray     #4                  // class java/lang/Object',
        '      10: dup', '      11: iconst_0', '      12: aload_1', '      13: aastore',
        '      14: dup', '      15: iconst_1', '      16: aconst_null', '      17: aastore',
        '      18: invokevirtual #5                  // Method scala/runtime/ScalaRunTime$.genericWrapArray:(Ljava/lang/Object;)Lscala/collection/immutable/ArraySeq;',
        '      21: invokevirtual #6                  // Method scala/runtime/TupleXXL$.apply:(Lscala/collection/immutable/Seq;)Lscala/runtime/TupleXXL;',
        '      24: astore_2', '      25: aload_2', '      26: astore_3', '      27: aload_3', '      28: iconst_0',
        '      29: invokevirtual #7                  // Method scala/runtime/TupleXXL.productElement:(I)Ljava/lang/Object;',
        '      32: checkcast     #8                  // class scala/Tuple2',
        '      35: astore        4', '      37: aload_2', '      38: astore        5', '      40: aload         5', '      42: iconst_1',
        '      43: invokevirtual #7                  // Method scala/runtime/TupleXXL.productElement:(I)Ljava/lang/Object;',
        '      46: invokestatic  #9                  // Method scala/runtime/BoxesRunTime.unboxToInt:(Ljava/lang/Object;)I',
        '      49: istore        6', '      51: return']
    seg_teq = [l for l in seg_scalac]
    seg_teq[1:2] = ['       3: new           #10                 // class scala/collection/immutable/ArraySeq$ofRef', '       4: dup']
    k = next(i for i, l in enumerate(seg_teq) if 'genericWrapArray' in l)
    seg_teq[k:k + 1] = ['      18: invokespecial #11                 // Method scala/collection/immutable/ArraySeq$ofRef."<init>":([Ljava/lang/Object;)V',
                        '      19: checkcast     #12                 // class scala/collection/immutable/Seq']

    def segment(change=None):
        def plant(d):
            for side in ('scalac', 'teq', 'ref'):
                code = list(seg_scalac if side == 'scalac' else seg_teq)
                if side == 'teq' and change:
                    code = change(code)
                p = os.path.join(d, 'out', f'{side}.javap')
                text = open(p).read()
                head = '  public void c_tup_tail_int(scala.Tuple2);\n    Code:\n'
                start = text.index(head) + len(head)
                end = text.index('  public void c_single_tail_int')
                open(p, 'w').write(text[:start] + '\n'.join(code) + '\n' + text[end:])
        return plant

    def replaced(old, new):
        return lambda code: [new if l.strip() == old else l for l in code]

    def dropped(*olds):
        return lambda code: [l for l in code if l.strip() not in olds]

    def swapped(a, b):
        return lambda code: [b if l.strip() == a else a if l.strip() == b else l for l in code]

    def edit(side, kind, old, new):
        def plant(d):
            p = os.path.join(d, 'out', f'{side}.{kind}')
            text = open(p).read()
            assert old in text, (side, kind, old)
            open(p, 'w').write(text.replace(old, new))
        return plant

    def panicked(side):
        return lambda d: open(os.path.join(d, 'out', f'{side}.build'), 'w').write("thread 'main' panicked at src/jvm/lower.rs:10:5:\nexplicit panic\n")

    record('the control', True, 3, lambda d: None, want_panics=0)
    record('the slots renumbered alike, every read the same store\'s', True, 3,
           lambda d: (edit('teq', 'javap', '7: istore_3', '7: istore        5')(d), edit('teq', 'javap', '11: iload_3', '11: iload         5')(d)))
    record('two variables\' reads swapped (judged as differing)', True, 2,
           lambda d: (edit('teq', 'javap', '11: iload_3', '11: iload         4')(d), edit('teq', 'javap', '12: iload         4', '12: iload_3')(d)))
    record('a jump to another place (judged as differing)', True, 2, edit('teq', 'javap', 'ifnull        16', 'ifnull        15'))
    record('a lambda\'s body changed, its outer method not (judged as differing)', True, 2, edit('teq', 'javap', '1: iconst_3', '1: iconst_4'))
    record('a lambda\'s bootstrap naming another body (judged as differing)', True, 2, edit('teq', 'javapv', 'PatDef$.$anonfun$7', 'PatDef$.$anonfun$8'))
    record('a value moved', False, None, edit('teq', 'out', 'tup_tail_int 1 2', 'tup_tail_int 2 1'))
    record('a cell\'s code missing on the teq side', False, None, edit('teq', 'javap', 'c_tup_tail_int', 'c_other'))
    record('a cell missing from a run', False, None, edit('teq', 'out', 'single_tail_int 1\n', ''))
    record('a cell printed twice', False, None, edit('scalac', 'out', 'single_tail_int 1\n', 'single_tail_int 1\nsingle_tail_int 1\n'))
    record('a cell listed twice', False, None, lambda d: open(os.path.join(d, 'cells.txt'), 'a').write('tup_tail_int tup tail int MatchableTuple\n'))
    record('a panic in teq\'s build', False, None, panicked('teq'), want_panics=1)
    record('a panic in the reference\'s build', False, None, panicked('ref'), want_panics=1)
    record('a cell of a rung not taken moved from the reference', False, None, edit('teq', 'javap', 'c_single_tail_int(int);\n    Code:\n       0: return', 'c_single_tail_int(int);\n    Code:\n       0: nop\n       1: return'))
    record('control: a cell of a rung taken moved from the reference (judged by scalac alone)', True, 3, edit('ref', 'javap', '6: iconst_1', '6: iconst_0'))
    record('control: a tuple past 22 made of teq\'s varargs argument (the varargs argument\'s form alone)', True, 2, made_xxl(), want_varargs=1)
    record('a tuple past 22 made of teq\'s varargs argument over another element (judged as differing)', True, 2, made_xxl(('13: aload_1', '13: aload_2')), want_varargs=0)
    record('a tuple past 22 of teq\'s varargs argument without its cast to Seq (judged as differing)', True, 2,
           made_xxl(('      18: checkcast     #9                  // class scala/collection/immutable/Seq\n', '')), want_varargs=0)
    record('control: the tuple past 22\'s build and reads but for the varargs argument\'s form', True, 2, segment(), want_segments=(1, 0, 1, 0, 0))
    record('a tuple past 22\'s read off by one (its segment judged as differing)', True, 2,
           segment(replaced('42: iconst_1', '      42: iconst_0')), want_segments=(1, 0, 0, 0, 1))
    record('a tuple past 22\'s read without its cast (its segment judged as differing)', True, 2,
           segment(dropped('32: checkcast     #8                  // class scala/Tuple2')), want_segments=(1, 0, 0, 0, 1))
    record('a tuple past 22\'s read without its unbox (its segment judged as differing)', True, 2,
           segment(dropped('46: invokestatic  #9                  // Method scala/runtime/BoxesRunTime.unboxToInt:(Ljava/lang/Object;)I')), want_segments=(1, 0, 0, 0, 1))
    record('a tuple past 22\'s elements out of order in its array (its segment judged as differing)', True, 2,
           segment(swapped('12: aload_1', '16: aconst_null')), want_segments=(1, 0, 0, 0, 1))
    record('a tuple past 22\'s segment cut short (judged as differing)', True, 2,
           segment(dropped('37: aload_2', '38: astore        5', '40: aload         5', '42: iconst_1',
                           '43: invokevirtual #7                  // Method scala/runtime/TupleXXL.productElement:(I)Ljava/lang/Object;',
                           '46: invokestatic  #9                  // Method scala/runtime/BoxesRunTime.unboxToInt:(Ljava/lang/Object;)I',
                           '49: istore        6')), want_segments=(1, 0, 0, 0, 1))
    for label, right, last in results:
        print(f'{"right" if right else "WRONG"} {label}: {last[:140]}')
    wrong = sum(1 for _, r, _ in results if not r)
    print(f'self-test patdef table: {len(results)} records (5 controls, {len(results) - 5} planted bad), {wrong} wrong')
    return 1 if wrong else 0


def main():
    a = sys.argv[1:]
    if len(a) == 2 and a[0] == 'gen':
        return gen(a[1])
    if len(a) == 2 and a[0] == 'judge':
        return judge(a[1])
    if len(a) == 2 and a[0] == 'selftest':
        return selftest(a[1])
    print(__doc__)
    return 2


sys.exit(main())
