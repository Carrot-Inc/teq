// A by-name argument reaches a macro's reflection as its expression, a named one inside its `NamedArg`, of a method
// and of a constructor alike, and one tree each time the macro takes the application apart (dotty compares trees by
// reference). scalac prints the lines of the .expected file.
object F:
  def c2(k: Int)(a: Int, b: => Int): Int = k + b * 10 + a
class A(a: Int, b: => Int):
  def v = b * 10 + a
def x(s: String, v: Int) = { println(s); v }
@main def run(): Unit =
  println(Mac.probe(F.c2(100)(1, b = x("b", 2))))
  println(Mac.probe(new A(1, x("b", 2))))
  println(Mac.probe(new A(1, b = x("b", 2))))
  println(Mac.probe(F.c2(100)(b = 2, a = 1)))
