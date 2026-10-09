// A failed lexical application's using clauses are not resolved again: the transparent given's
// macro runs once (-1, 2), as scalac and master print.
class C
object C:
  extension (c: C) def f(k: Int): Int = -k
class Missing
extension (c: C)(using x: X, m: Missing) def f(k: Int): Int =
  x.v + k
@main def run(): Unit =
  println(C().f(1))
  println(Counter.next)
