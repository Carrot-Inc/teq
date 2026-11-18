// expect: 7:33: error: expected end of statement, found '('
// expect: 10:5: error: expected end of statement, found '('
// The operand of `return` is an expression no argument list applies: after a block in braces as the operand, an
// argument list on the same line, or on a line further in, is refused. scalac: "end of statement expected but
// '(' found" at both.
final class V(val s: String) { def apply(o: V): V = V(s + "(" + o.s + ")") }
def t: Any = { return { V("a") }(V("p")) }
def u: Any =
  return { V("a") }
    (V("p"))
