import java.nio.CharBuffer

// A wrapped character sequence's length when wrapped is the buffer's capacity, which the
// sequence growing afterwards leaves as it is.
@main def main(): Unit =
  val s = new java.lang.StringBuilder("a")
  val b = CharBuffer.wrap(s)
  s.append("b")
  println(s"${b.capacity()} ${b.limit()}")
  b.clear()
  println(s"${b.limit()} ${b.subSequence(0, 1).capacity()}")
  try b.limit(2) catch case _: IllegalArgumentException => println("limit past capacity")
