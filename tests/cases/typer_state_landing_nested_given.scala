// A nested application's search does not take the given a lexical extension's probe resolved
// for the application around it: the local given answers `summon[X]` inside the argument (49),
// as scalac and master print.
class X(val v: Int)
object X:
  given x: X = new X(4)
class C
object C:
  extension (c: C) def f(k: Int): Int = -k
extension (c: C) def f(k: Int)(using x: X): Int =
  x.v * 10 + k
@main def run(): Unit =
  println(C().f({ given local: X = new X(9); summon[X].v }))
