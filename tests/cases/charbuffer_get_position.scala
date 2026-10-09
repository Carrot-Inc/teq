import java.nio.CharBuffer

// A relative `get` moves the position before it reads, so a read the wrapped sequence fails
// leaves the position past it, as the JDK's does.
@main def main(): Unit =
  val s = CharBuffer.wrap(Array('a'))
  val b = CharBuffer.wrap(s: CharSequence)
  s.get()
  try b.get() catch case _: IndexOutOfBoundsException => println("out of bounds")
  println(b.position())
