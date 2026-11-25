#!/usr/bin/env python3
"""tests/right-assoc/lines.py: the one place a cell's result becomes a line, for scalac's capture and for teq's legs.
  run <cell> [exit]   stdin: a program's output -> `<cell>\trun\t<lines joined by ' | '>` where the run exited 0;
                      `<cell>\tcrash <exit>\t` for any other exit and `<cell>\ttimeout\t` for one cut at its bound,
                      whatever it printed, which no leg is judged against
  refused <cell>      stdin: a compiler's diagnostics (scalac's boxes or teq's `file:line:col: error: msg`) ->
                      `<cell>\trefused\t<line>:<column from 1>: <first line of the first error's message>`
  lib                 stdin: a cell's Test.scala -> the definitions above `@main def run` (empty for a cell with none)
  descriptors <cell>  stdin: javap -s -p -> `<cell>\t<name descriptor> ; ...` for the methods whose names hold the
                      operators' encoded names
  merge <out> <expected.tsv> <descriptors.tsv> [prefix ...]: the capture's lines into the two files, keeping the
                      lines of the cells the prefixes leave out
  --self-test         the parsers on planted inputs"""
import os, re, sys

OPS = ('$plus$colon', '$colon$colon', 'x_$colon', '$hash$colon', '$times$colon', '$plus$eq$colon', '$div$colon')


def run_line(cell, text, code=0):
    if code == 'timeout':
        return f'{cell}\ttimeout\t'
    if code != 0:
        return f'{cell}\tcrash {code}\t'
    return f'{cell}\trun\t' + ' | '.join(l.rstrip() for l in text.strip().splitlines())


def refused_line(cell, text):
    """scalac's first `-- ... Error: f:L:C ---` box (its column from 0) and the box's first message line, or teq's
    first `f:L:C: error: msg` (its column from 1)."""
    lines = text.splitlines()
    for i, l in enumerate(lines):
        m = re.match(r'^-- (?:\[E\d+\] )?.*Error: .*?:(\d+):(\d+) -+', l)
        if m:
            line, col = int(m.group(1)), int(m.group(2)) + 1
            for b in lines[i + 1:]:
                if b.startswith('-- '):
                    break
                s = re.sub(r'^\s*(?:\d+\s*)?\|\s?', '', b)
                if b.lstrip()[:1] in ('|',) and s.strip() and not re.fullmatch(r'\s*\^+\s*', s) and not re.match(r'^\d+ \|', b.strip()):
                    return f'{cell}\trefused\t{line}:{col}: {s.strip()}'
            return f'{cell}\trefused\t{line}:{col}: '
        m = re.match(r'^.*?:(\d+):(\d+): error: (.*)$', l)
        if m:
            return f'{cell}\trefused\t{m.group(1)}:{m.group(2)}: {m.group(3).strip()}'
    return f'{cell}\trefused\t?: ' + (lines[0].strip() if lines else '')


def lib(text):
    i = text.find('@main def run')
    return text[:i] if i > 0 else ''


def descriptors(cell, text):
    out, name = set(), None
    for line in text.splitlines():
        m = re.search(r'\s([\w$]+)\(.*\);$', line)
        if m:
            name = m.group(1)
            continue
        d = re.match(r'\s+descriptor: (.*)$', line)
        if d and name and any(op in name for op in OPS):
            out.add(f'{name} {d.group(1)}')
            name = None
    return f'{cell}\t' + ' ; '.join(sorted(out)) if out else ''


def merge(out, expected, descs, prefixes):
    def keep(path):
        if not prefixes or not os.path.exists(path):
            return []
        return [l for l in open(path).read().splitlines() if l and not l.startswith('#') and not any(l.startswith(p) for p in prefixes)]
    exp, des = keep(expected), keep(descs)
    work = os.path.join(out, 'work')
    for cell in sorted(os.listdir(work)):
        for name, acc in (('line', exp), ('desc', des)):
            p = os.path.join(work, cell, name)
            if os.path.exists(p):
                s = open(p).read().strip()
                if s:
                    acc.append(s)
    head = '# tests/right-assoc/capture.sh writes this file: scalac 3.8.4\'s line of each cell of tests/right-assoc/gen.py.\n'
    open(expected, 'w').write(head + '\n'.join(sorted(exp)) + '\n')
    open(descs, 'w').write(head.replace("line of each cell", "descriptors of each cell's definitions") + '\n'.join(sorted(des)) + '\n')
    print(f'right-assoc capture: {len(exp)} lines, {len(des)} descriptor lines')


def self_test():
    box = ('-- [E006] Not Found Error: /w/Test.scala:4:18 -----------\n4 |  def +:(r: Int = l): Int = l + r\n'
           '  |                  ^\n  |                  Not found: l\n  |\n  | longer explanation available\n1 error found\n')
    assert refused_line('c', box) == 'c\trefused\t4:19: Not found: l', refused_line('c', box)
    teq = '/w/Test.scala:4:19: error: Not found: l\n    def +:(r: Int = l)\n1 error found\n'
    assert refused_line('c', teq) == 'c\trefused\t4:19: Not found: l'
    assert run_line('c', 'a\nb  \n') == 'c\trun\ta | b'
    assert run_line('c', 'a\nb\n', 1) == 'c\tcrash 1\t'
    assert run_line('c', 'a\n', 'timeout') == 'c\ttimeout\t'
    jp = '  public java.lang.String $plus$colon(java.lang.String, Num);\n    descriptor: (Ljava/lang/String;LNum;)Ljava/lang/String;\n  public int other();\n    descriptor: ()I\n'
    assert descriptors('c', jp) == 'c\t$plus$colon (Ljava/lang/String;LNum;)Ljava/lang/String;'
    assert lib('object A\n@main def run(): Unit = ()\n') == 'object A\n'
    return 7


if __name__ == '__main__':
    a = sys.argv[1:]
    if a[:1] == ['--self-test']:
        print(f'lines: self-test: {self_test()} of 7 planted inputs read as planted')
    elif a[0] == 'run':
        code = a[2] if len(a) > 2 else '0'
        print(run_line(a[1], sys.stdin.read(), code if code == 'timeout' else int(code)))
    elif a[0] == 'refused':
        print(refused_line(a[1], sys.stdin.read()))
    elif a[0] == 'lib':
        sys.stdout.write(lib(sys.stdin.read()))
    elif a[0] == 'descriptors':
        s = descriptors(a[1], sys.stdin.read())
        if s:
            print(s)
    elif a[0] == 'merge':
        merge(a[1], a[2], a[3], a[4:])
