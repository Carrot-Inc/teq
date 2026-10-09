//> using platform js
// A chain of `+` in a method whose result type is inferred, typed when an inline method that
// calls it expands.
inline def callAll(inline l: Any): Unit =
  println("plain"); println(Helpers.plain(l))
  println("constant if"); println(Helpers.constantIf(l))
  println("final val"); println(Helpers.viaFinal(l))
  println("ascribed"); println(Helpers.ascribed(l))
  println("toString"); println(Helpers.shown(l))
  println("cast"); println(Helpers.cast(l))
  println("transparent head"); println(Helpers.transparentHead(l))
  println("plain inline head"); println(Helpers.plainHead(l))
  println("block"); println(Helpers.block(l))

inline def nested(inline l: Any): String = Helpers.constantIf(l) + tail()

@main def main(): Unit =
  val l = new Loud("x")
  callAll(l)
  println("a demanded body's chain as a head"); println(nested(l))
