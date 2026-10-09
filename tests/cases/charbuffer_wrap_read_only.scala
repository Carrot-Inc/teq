import java.nio.{CharBuffer, ReadOnlyBufferException}

// `CharBuffer.wrap` of a character sequence is read-only, and so are its views; of an array it
// is writable.
@main def main(): Unit =
  val r = CharBuffer.wrap("ab")
  println(s"${r.isReadOnly()} ${CharBuffer.wrap(Array('a')).isReadOnly()}")
  try r.put('x') catch case _: ReadOnlyBufferException => println("read-only")
  try r.subSequence(0, 1).put('y') catch case _: UnsupportedOperationException => println("read-only view")
  println(r.toString)
