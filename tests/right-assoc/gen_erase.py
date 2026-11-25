#!/usr/bin/env python3
"""gen_erase.py <out dir>: the companion refusal's cells whose two operands ERASE ALIKE, one cell a program, named as
gen_refusal.py's (`rf_<kind>_<companion>_<clauses>_<access>_<operands>`), so that judge_refusal.py's rule reads them:
the value class over Int with Int operands (`int`), over String with String operands (`str`), generic over A with Any
operands (`gen`) x its companion with a statement (`stmt`, refused where an operand is no literal constant) or quiet
(`quiet`, written) x one clause, two x infix through the instance's import, written out x operands pure (literal
constants), impure (calls with effects), mixed (a call left, a constant right). Where the two erase alike, a class
that takes them in the other order loads and runs with the values swapped."""
import os, sys

KINDS = {
    'int': ('class V(val n: Int) extends AnyVal', 'Int', 'Int', 'n', 'new V(7)', ('num(1)', '1'), ('num(2)', '2')),
    'str': ('class V(val s: String) extends AnyVal', 'String', 'String', 's', 'new V("v")', ('note("a")', '"a"'), ('note("b")', '"b"')),
    'gen': ('class V[A](val a: A) extends AnyVal', 'Any', 'Any', 'a', 'new V("g")', ('num(1)', '1'), ('note("b")', '"b"')),
}
COMPANIONS = {'stmt': 'object V { Main.log = Main.log :+ "c" }', 'quiet': 'object V { def helper: Int = 1 }'}
CLAUSES = {'one': ('', '', '{core}'), 'two': ('(z: Int)', ' + "|" + z', '({core})(3)')}
CELLS = {'pure': (1, 1), 'impure': (0, 0), 'mixed': (0, 1)}

os.makedirs(sys.argv[1], exist_ok=True)
n = 0
for kind, (cls, xt, yt, field, make, lops, rops) in KINDS.items():
    for comp, companion in COMPANIONS.items():
        for clauses, (tail, extra, wrap) in CLAUSES.items():
            for access in ('infix', 'written'):
                for cell, (li, ri) in CELLS.items():
                    l, r = lops[li], rops[ri]
                    name = f'rf_{kind}_{comp}_{clauses}_{access}_{cell}'
                    core = f'{l} +: {r}' if access == 'infix' else f'v.+:({r})({l})'
                    body = [f'val v = {make}', 'log = Nil'] + (['import v.*'] if access == 'infix' else []) + [f'val res: String = {wrap.format(core=core)}', f'println("c_{cell} " + res + " " + log.mkString(","))']
                    text = f'''// {name}: the companion refusal's cell whose operands erase alike (gen_erase.py).
{cls} {{
  extension (x: {xt}) def +:(y: {yt}){tail}: String = "" + y + "|" + x{extra} + "|" + {field}
}}
{companion}

object Main {{
  var log = List.empty[String]
  def num(k: Int): Int = {{ log = log :+ ("n" + k); k }}
  def note(s: String): String = {{ log = log :+ s; s }}
  def c_{cell}(): Unit = {{
    {chr(10).join("    " + b if i else b for i, b in enumerate(body))}
  }}
  def main(args: Array[String]): Unit =
    try c_{cell}() catch {{ case e: Throwable => println("c_{cell} !" + e.getClass.getName) }}
}}
'''
                    open(os.path.join(sys.argv[1], name + '.scala'), 'w').write(text)
                    n += 1
print(n, 'programs')
