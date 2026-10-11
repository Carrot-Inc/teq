// A right-associative extension's default getter takes the clauses in the declaration's order, the
// method's first before the receiver's (`Desugar.extMethod`), whatever order the call is written in;
// a repeated argument the plan binds stays the wrapped array the call passes.
object Ops:
  def s(x: String): String = { print(x); x }
  extension (r: String) def ++:(a: String)(b: String = a + "/" + r): String = b
  extension (r: String) def +/:(a: String = "d")(b: String = a + "|" + r): String = b
  def kinds(xs: Int*)(b: Int = xs.size): String = (if xs.isInstanceOf[List[?]] then "list" else "array") + b
  def n(s: String): Int = { print(s); 1 }

@main def main(): Unit =
  import Ops.*
  println((s("A") ++: s("R"))())
  println(Ops.++:(s("a"))(s("r"))())
  println(("x" ++: "y")(b = "given"))
  println(Ops.+/:()(s("q"))())
  println(kinds(n("A"), n("B"))())
  println(kinds()())
