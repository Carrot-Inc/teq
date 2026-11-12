// The warning of a transparent given a lexical extension's probe resolved goes with the
// application that takes it: scalac reports the warning and 41.
class C
object C:
  extension (c: C) def f(k: Int): Int = -k
extension (c: C)(using x: X) def f(k: Int): Int =
  x.v * 10 + k
@main def run(): Unit = println(C().f(1))
