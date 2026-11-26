#!/usr/bin/env python3
"""tests/default-getters/diamond.py: the second enumerated family's STRUCTURAL rule (scalac's lifted getters get no
forwarder). Mixin writes no forwarder for a local trait's getter once LambdaLift has lifted it, so a class whose own
chain (the class and its superclasses) declares no getter for a method's default dispatches to the JVM's default
methods. Where two or more of its local traits declare that getter with a default reading the block's values (a lifted
getter) and neither derives from the other (the JVM's
maximally specific declarers), the JVM throws IncompatibleClassChangeError: a valid program that scalac's own run
cannot run.
  unforwarded_diamond(text)   whether a program holds such a class: a block's (a def's body's) traits and classes,
                              their parents, which declare a default for which method; for each class, its chain's
                              declarers, then the traits' maximal declarers.
  diamond.py --self-test      the rule on planted programs."""
import re, sys

DEF = re.compile(r'^\s*(abstract\s+)?(trait|class)\s+(\w+)(?:\s+extends\s+([\w\s]+?))?\s*(\{.*)?$')


def blocks(text):
    """The statements of every def's block (`def ... = {` to its matching brace), split at the block's own depth on
    `;` and newlines."""
    out = []
    for m in re.finditer(r'def \w+\([^)]*\)[^={]*=\s*\{', text):
        i, depth, stmts, cur = m.end(), 1, [], []
        while i < len(text) and depth:
            ch = text[i]
            if ch == '{':
                depth += 1
            elif ch == '}':
                depth -= 1
                if depth == 0:
                    break
            if depth == 1 and ch in ';\n':
                stmts.append(''.join(cur))
                cur = []
            else:
                cur.append(ch)
            i += 1
        stmts.append(''.join(cur))
        out.append([s for s in stmts if s.strip()])
    return out


def declares_default(text, locals_):
    """The methods a definition's body declares a LIFTED default for: `def m(... = <e>)` whose default reads a value of
    the block (one of `locals_`), the getter LambdaLift gives the block's values as parameters. A default reading none
    is an ordinary getter, which Mixin forwards."""
    out = set()
    for m in re.finditer(r'def (\w+)\(([^)]*)\)', text):
        defaults = [p.split('=', 1)[1] for p in m.group(2).split(',') if '=' in p]
        if defaults and locals_ is None:
            out.add(m.group(1))  # any default: a class's own getter, which overrides the traits' wherever it is
        elif locals_ is not None and any(re.search(r'\b' + re.escape(v) + r'\b', d) for d in defaults for v in locals_):
            out.add(m.group(1))
    return out


def unforwarded_diamond(text):
    for body in blocks(text):
        defs = {}
        locals_ = {m.group(1) for l in body for m in [re.match(r'^\s*(?:val|var|lazy val|def)\s+(\w+)', l)] if m}
        for l in body:
            m = DEF.match(l)
            if not m:
                continue
            kind, name, parents, rest = m.group(2), m.group(3), m.group(4) or '', m.group(5) or ''
            defs[name] = (kind, [p.strip() for p in parents.split(' with ') if p.strip()], declares_default(rest, locals_),
                          declares_default(rest, None))

        def ancestors(n, seen=None):
            seen = set() if seen is None else seen
            for p in defs.get(n, (None, [], set(), set()))[1]:
                if p in defs and p not in seen:
                    seen.add(p)
                    ancestors(p, seen)
            return seen
        for name, (kind, parents, own, own_any) in defs.items():
            if kind != 'class':
                continue
            chain, k = [name], name
            while True:
                sup = [p for p in defs.get(k, (None, [], set(), set()))[1][:1] if p in defs and defs[p][0] == 'class']
                if not sup:
                    break
                k = sup[0]
                chain.append(k)
            anc = ancestors(name)
            traits = [t for t in anc if defs[t][0] == 'trait']
            for meth in set().union(*(defs[t][2] for t in traits)) if traits else set():
                if any(meth in defs[c][3] for c in chain):
                    continue
                declarers = [t for t in traits if meth in defs[t][2]]
                maximal = [t for t in declarers if not any(t in ancestors(u) for u in declarers if u != t)]
                if len(maximal) >= 2:
                    return True
    return False


def def_bodies(text):
    """Each def's body, in brace syntax (to its matching brace) or indentation syntax (the lines indented under it)."""
    out = [' ; '.join(b) for b in blocks(text)]
    lines = text.split('\n')
    for i, l in enumerate(lines):
        if re.match(r'^\s*def \w+.*=\s*$', l):
            ind = len(l) - len(l.lstrip())
            body = []
            for k in lines[i + 1:]:
                if k.strip() and len(k) - len(k.lstrip()) <= ind:
                    break
                body.append(k)
            out.append('\n'.join(body))
    return out


def lifted_member_override(text):
    """The first family's member form: a local trait whose method reads a value of its block (LambdaLift gives the
    method that value as a parameter) overridden by an anonymous class of the same block (`new T { override def m`),
    whose method takes no such parameter: the override is lost under scalac."""
    for body in def_bodies(text):
        locals_ = set(re.findall(r'(?:^|[;\n])\s*(?:val|var|lazy val)\s+(\w+)', body))
        for t_ in re.finditer(r'trait (\w+)[^{]*\{(.*?)\}', body, re.S):
            name, tbody = t_.group(1), t_.group(2)
            for m in re.finditer(r'def (\w+)\(([^)]*)\)[^=]*=([^;\n}]*)', tbody):
                reads = any(re.search(r'\b' + re.escape(v) + r'\b', m.group(2) + m.group(3)) for v in locals_)
                if reads and re.search(r'new ' + name + r'\s*\{\s*override def ' + m.group(1) + r'\(', body):
                    return True
    return False


def moves_state(text):
    """State a member twin would move: a captured var, a lazy or by-name default, a forward reference's shape."""
    return any(re.search(r'\bvar\b|\blazy\b|:\s*=>', s) for b in blocks(text) for s in b)


def member_twin(text):
    """The program with its diamond's block made members: object Main's own statements kept (but the def holding the
    block and main), the block's vals, traits and classes moved to object Main, the rest of the block a def `go`."""
    m = re.search(r'object Main\s*\{', text)
    if not m:
        return None
    i, depth, stmts, cur = m.end(), 1, [], []
    while i < len(text) and depth:
        ch = text[i]
        if ch == '{':
            depth += 1
        elif ch == '}':
            depth -= 1
            if depth == 0:
                break
        if depth == 1 and ch in ';\n':
            stmts.append(''.join(cur).strip())
            cur = []
        else:
            cur.append(ch)
        i += 1
    stmts.append(''.join(cur).strip())
    stmts = [s for s in stmts if s]
    holder = next((s for s in stmts if re.match(r'def \w+\(.*?\)[^={]*=\s*\{', s) and DEF.search(s) is None and 'trait ' in s), None)
    if holder is None:
        return None
    block = blocks(holder)[0]
    moved = [s.strip() for s in block if re.match(r'^\s*(val|var|lazy val|trait|class|abstract class|object)\b', s)]
    kept = [s.strip() for s in block if s.strip() not in moved]
    own = [s for s in stmts if s is not holder and not s.startswith('def main(')]
    body = '\n'.join('  ' + s for s in own + moved)
    return f'object Main {{\n{body}\n  def go(): Unit = {{\n' + '\n'.join('    ' + s for s in kept) + '\n  }\n  def main(args: Array[String]): Unit = go()\n}\n'


def self_test():
    head = 'object Main {\n  def run(): Unit = {\n    val y = 10\n    trait A { def m(a: Int = y): Int = a }\n'
    planted = {
        'diamond': (head + '    trait B extends A { override def m(a: Int = y + 1): Int = a }\n    trait D extends A { override def m(a: Int = y + 2): Int = a }\n    class C extends B with D\n  }\n}\n', True),
        'one drops': (head + '    trait B extends A { override def m(a: Int = y + 1): Int = a }\n    trait D extends A { override def m(a: Int): Int = a }\n    class C extends B with D\n  }\n}\n', False),
        'a line': (head + '    trait B extends A { override def m(a: Int = y + 1): Int = a }\n    class C extends A with B\n  }\n}\n', False),
        'the class declares': (head + '    trait B extends A { override def m(a: Int = y + 1): Int = a }\n    trait D extends A { override def m(a: Int = y + 2): Int = a }\n    class C extends B with D { override def m(a: Int = 3): Int = a }\n  }\n}\n', False),
        'a join declaring': (head + '    trait B extends A { override def m(a: Int = y + 1): Int = a }\n    trait D extends A { override def m(a: Int = y + 2): Int = a }\n    trait E extends B with D { override def m(a: Int = y + 3): Int = a }\n    class C extends E\n  }\n}\n', False),
        'a join dropping': (head + '    trait B extends A { override def m(a: Int = y + 1): Int = a }\n    trait D extends A { override def m(a: Int = y + 2): Int = a }\n    trait E extends B with D { override def m(a: Int): Int = a }\n    class C extends E\n  }\n}\n', True),
        'unlifted getters': ('object Main {\n  def run(): Unit = {\n    trait A { def m(a: Int = 10): Int }\n    trait B { def m(a: Int = 20): Int }\n    class C extends A with B { override def m(a: Int): Int = a }\n  }\n}\n', False),
    }
    bad = [n for n, (t, want) in planted.items() if unforwarded_diamond(t) != want]
    if bad:
        sys.exit('diamond SELF-TEST FAILED: ' + ', '.join(bad))
    print(f'diamond self-test: {len(planted)} planted programs read right (a diamond, a drop, a line, the class declaring, a join declaring, a join dropping, unlifted getters)')


if __name__ == '__main__':
    self_test()
