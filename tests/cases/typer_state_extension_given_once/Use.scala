// A lexical extension beside a companion's whose using clause a transparent given fills: the
// lexical candidate is applied once, its given resolved by that application alone (scalac `11`,
// `2`; a selection test ahead of the application resolved it twice, `21`, `3`).
class C
object C:
  extension (c: C) def f(k: Int): Int = -k

extension (c: C)(using x: X) def f(k: Int): Int = x.v * 10 + k

@main def run(): Unit =
  println(C().f(1))
  println(Counter.next)
