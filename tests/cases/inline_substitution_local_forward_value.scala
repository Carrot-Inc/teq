// A local inline method called before its block reaches its definition, across an import from
// a value the call's scope already binds (`import source.k` over a `source` defined before the
// call), reads the import as the block enters it: `k` is `source.k`, as scalac 3.8.4 types it,
// for the call before the definition and the one after (2, 2), a renamed member the same (3).
class Box:
  val k = 2
  val m = 3

@main def run(): Unit =
  val source = new Box
  val first = g()
  import source.k
  import source.{m as n}
  inline def g(): Int = k
  inline def h(): Int = n
  println(first)
  println(g())
  println(h())
