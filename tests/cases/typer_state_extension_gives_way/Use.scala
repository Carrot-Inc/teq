// A lexical extension whose using clause has no instance gives way to the companion's: its
// application's argument, a macro's call, is typed once for both candidates (scalac `-1`, `2`).
class C
object C:
  extension (c: C) def g(k: Int): Int = -k

class Missing

extension (c: C)(using m: Missing) def g(k: Int): Int = k

@main def run(): Unit =
  println(C().g(Counter.next))
  println(Counter.next)
