// A by-name parameter of an inline method takes its argument's expression when the call names its
// arguments out of order (`Inliner.paramBindingDef`'s `dropNameArg`): each by-value argument is
// evaluated once in the order written, each by-name one where and as often as the body reads it.
// scalac prints the lines of the .expected file.
object M:
  inline def f(a: Int, b: => Int): Int = b * 10 + a
  inline def g(a: => Int, b: Int, c: => Int): Int = c * 100 + b * 10 + a
  inline def twice(a: Int, b: => Int): Int = b + b + a
  inline def inlined(a: Int, inline b: => Int): Int = a + b + b
var log = List.empty[String]
def x(s: String, v: Int): Int = { log = s :: log; v }
@main def run(): Unit =
  println(M.f(b = x("b", 2), a = x("a", 1)))
  println(log.reverse); log = Nil
  println(M.g(c = x("c", 3), a = x("a", 1), b = x("b", 2)))
  println(log.reverse); log = Nil
  println(M.twice(b = x("b", 2), a = x("a", 1)))
  println(log.reverse); log = Nil
  println(M.inlined(b = x("b", 2), a = x("a", 1)))
  println(log.reverse); log = Nil
  println(M.f(b = 2, a = 1))
