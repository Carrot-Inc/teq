// A by-name argument reaches a macro's reflection as its expression, named or not: `F.f(1, b = e)` holds
// `NamedArg(b, e)`, which the macro reads through. scalac prints the lines of the .expected file.
object F:
  def f(a: Int, b: => Int): Int = b * 10 + a
def x(s: String, v: Int) = { println(s); v }
@main def run(): Unit =
  println(Mac.sumArgs(F.f(1, x("p", 2))))
  println(Mac.sumArgs(F.f(1, b = x("n", 2))))
