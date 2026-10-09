import java.nio.{CharBuffer, BufferUnderflowException, BufferOverflowException}

// Relative `get` and `put` stop at the limit, as the JDK's throw.
@main def main(): Unit =
  val b = CharBuffer.wrap(Array('a', 'b'))
  b.limit(1)
  println(b.get())
  try println(b.get()) catch case _: BufferUnderflowException => println("underflow")
  try b.put('x') catch case _: BufferOverflowException => println("overflow")
  val c = CharBuffer.allocate(1)
  c.put('z')
  try c.put('y') catch case _: BufferOverflowException => println("full")
