// The warning of a transparent given a lexical extension's probe resolved goes with the
// application that takes it: the warning and 41, as scalac and master report.
class C
object C:
  extension (c: C) def f(k: Int): Int = -k
extension (c: C)(using x: X) def f(k: Int): Int =
  x.v * 10 + k
@main def run(): Unit = println(C().f(1))
