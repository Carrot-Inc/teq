// A given selector exports through a value the given alternative of an overloaded name alone, a
// stable member whose forwarder is the path through the value: read on a new instance, an unstable
// prefix, the path is the alternative itself, not the overloaded name.
object O:
  given x: Int = 1
  def x(s: String): String = s
  val y: Int = 2

class V:
  val o: O.type = O
  export o.given
  export o.y

@main def run(): Unit =
  println(V().x + 1)
  val i: Int = V().x
  println(i + V().y)
  println(O.x("s"))
