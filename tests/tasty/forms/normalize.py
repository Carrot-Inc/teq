#!/usr/bin/env python3
# tests/tasty/forms/normalize.py: the bodies' printout of `teq tasty --body --signatures <pickle> <class>` on
# stdin, normalised under the forms' equivalences, on stdout. Each rule
# is one of the table's equivalences; what no rule removes is a difference the twin's
# <name>.forms.diff pins, which the table classifies.
import re, sys

TYPE = r'(?:[^()]|\(\))+?'
RULES = [
    # paths: a class or an object named through its package object, its package or scala's root
    ('paths', re.compile(r'\b(?:_root_\.scala\.|scala\.package\.|package\$package\.|package\.|Predef\.|scala\.(?=[A-Z]))(?=[A-Za-z_$:])'), ''),
    # identifiers: a member of an enclosing class or object by name, or through its `this`
    ('identifiers', re.compile(r'\b[A-Za-z_][\w$]*\.this\.'), ''),
    # ascriptions: a `Typed` scalac's typer inserts around a name (avoidance, an expected type)
    ('ascriptions', re.compile(r'(?<![\w\]$)])(?<!case )\(([A-Za-z_][\w$]*): ' + TYPE + r'\)(?=[^\w\[(]|$)'), r'\1'),
    # blocks: a case's body as a block of no statements
    ('blocks', re.compile(r'=> \{ (.*) \}$'), r'=> \1'),
    # binders: a typed pattern's binder outside or inside the type test
    ('binders', re.compile(r'case \(([A-Za-z_][\w$]*(?: @ [^()]+)?: [^()]+)\)'), r'case \1'),
    ('binders', re.compile(r' @ \(([^()]*(?:\([^()]*\))?[^()]*)\)'), r' @ \1'),
    # type constructors: a type constructor argument and its eta-expansion
    ('constructors', re.compile(r'\[([A-Za-z_]\w*)\] =>> ([\w$.]+)\[\1\]'), r'\2'),
    ('constructors', re.compile(r'\[([A-Za-z_]\w*)\] =>> \(\) => \1\b'), 'Function0'),
    # fresh names: a wildcard type parameter's number
    ('fresh names', re.compile(r'_\$\d+'), '_$'),
]

def closing(line, i):
    '''The index of the bracket that closes the one at `i`, or None.'''
    pairs = {'(': ')', '[': ']', '{': '}'}
    stack = []
    for j in range(i, len(line)):
        c = line[j]
        if c in pairs:
            stack.append(pairs[c])
        elif stack and c == stack[-1]:
            stack.pop()
            if not stack:
                return j
    return None

HEAD = re.compile(r'(?<![\w$\])])\(([A-Za-z_][\w$]*)')

def selections(line):
    '''selections: a call written as a selection, parenthesised as one by the printer,
    `(f{sig}[T](args))` as `f{sig}[T](args)`, its brackets of any depth.'''
    changed = True
    while changed:
        changed = False
        out, i = [], 0
        while True:
            m = HEAD.search(line, i)
            if not m:
                break
            k = m.end()
            for open_ in '{[':
                if k < len(line) and line[k] == open_:
                    e = closing(line, k)
                    k = e + 1 if e is not None else len(line)
            e = closing(line, k) if k < len(line) and line[k] == '(' else None
            if e is None or e + 1 >= len(line) or line[e + 1] != ')':
                out.append(line[i:m.start() + 1])
                i = m.start() + 1
                continue
            out.append(line[i:m.start()])
            out.append(line[m.start() + 1:e + 1])
            i = e + 2
            changed = True
        out.append(line[i:])
        line = ''.join(out)
    return line

RULES.insert(next(i for i, r in enumerate(RULES) if r[0] == 'constructors'), ('selections', selections, None))

# A string or character literal, which no rule rewrites: `"A.this.x"` and `"x"` stay apart.
LITERAL = re.compile(r"'(?:[^'\\]|\\.)'" + r'|"(?:[^"\\]|\\.)*"')

def normalize(line):
    literals = []
    def mask(m):
        literals.append(m.group(0))
        return '"\x00%d\x00"' % (len(literals) - 1)
    line = LITERAL.sub(mask, line)
    for _, rx, rep in RULES:
        line = rx(line) if rep is None else rx.sub(rep, line)
    return re.sub(r'"\x00(\d+)\x00"', lambda m: literals[int(m.group(1))], line)

for line in sys.stdin:
    sys.stdout.write(normalize(line))
