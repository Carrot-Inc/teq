// A rejected lexical extension does not expand the arguments the companion's extension then
// expands again: the transparent macro runs once (-1, 2), as scalac and master print.
class C
object C:
  extension (c: C) def f(k: Int): Int = -k
class Missing
extension (c: C)(using m: Missing) def f(k: Int): Int = k
@main def run(): Unit =
  println(C().f(Counter.next))
  println(Counter.next)
