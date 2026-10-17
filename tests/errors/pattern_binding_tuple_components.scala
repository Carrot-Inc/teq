// A tuple pattern definition of variables, wildcards and typed ones is lowered as scalac 3.8.4's
// `Desugar.makePatDef` lowers it: a val per named component of its element, typed as written
// (`MatchableTuple`: `val accept: Accept = $1._2`, scalac reporting it where the pattern closes;
// `SimpleTuple` over a tuple literal: `val n: String = 2`). A typed component is a declared type,
// which `.runtimeChecked`, a parenthesised `: @unchecked` and a typed wildcard beside it do not
// exempt; a component in parentheses of its own takes the general match, and a literal adapts to
// its component's type, as scalac has them. An annotation on the right side in parentheses of its
// own is no `Annotated` tree to `makePatDef`, so its component is checked too.
class HttpHeader
class Accept extends HttpHeader
def parseLine(): (Int, HttpHeader) = (1, new Accept)
object Members:
  val (_, accept: Accept) = parseLine().runtimeChecked
  val (m, n: String) = (1, 2)
  val t = (3, 4)
  val (p, q: String) = t.runtimeChecked
def locals(): Unit =
  val (_, a: Accept) = parseLine().runtimeChecked
  val (_, b: Accept) = (parseLine(): @unchecked)
  val (_: Int, c: Accept) = parseLine().runtimeChecked
  val (k, (d: Accept)) = parseLine().runtimeChecked
  val (x, y: Long) = (1, 2)
  val (_, e: Accept) = (parseLine(): @scala.annotation.internal.RuntimeChecked)
  ()
// expect: 13:25: error: type mismatch: found HttpHeader, required Accept
// expect: 14:28: error: type mismatch: found 2, required String
// expect: 16:20: error: type mismatch: found Int, required String
// expect: 18:20: error: type mismatch: found HttpHeader, required Accept
// expect: 19:20: error: type mismatch: found HttpHeader, required Accept
// expect: 20:25: error: type mismatch: found HttpHeader, required Accept
// expect: 23:20: error: type mismatch: found HttpHeader, required Accept
// absent: pattern_binding_tuple_components.scala:21:
// absent: pattern_binding_tuple_components.scala:22:
