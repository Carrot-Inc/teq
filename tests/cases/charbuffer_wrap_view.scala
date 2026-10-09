import java.nio.CharBuffer

// `CharBuffer.wrap` of a character sequence reads the sequence as it stands, not a copy of it.
@main def main(): Unit =
  val a = Array('a', 'b')
  val source = CharBuffer.wrap(a)
  val b = CharBuffer.wrap(source: CharSequence)
  a(0) = 'x'
  println(s"${b.get()} ${b.capacity()} ${b.remaining()}")
  val sb = new java.lang.StringBuilder("hi")
  val c = CharBuffer.wrap(sb)
  sb.setCharAt(1, 'o')
  println(c.toString + " " + c.subSequence(1, 2))
