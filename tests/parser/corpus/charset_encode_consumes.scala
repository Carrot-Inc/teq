import java.nio.CharBuffer
import java.nio.charset.StandardCharsets

// `Charset.encode` of a buffer reads it to its limit.
@main def main(): Unit =
  val c = CharBuffer.wrap(Array('a', 'b', 'c'))
  c.limit(2)
  val bytes = StandardCharsets.UTF_8.encode(c)
  println(s"${c.position()} ${c.remaining()} ${bytes.remaining()}")
  val d = CharBuffer.wrap("hé")
  println(s"${StandardCharsets.UTF_8.encode(d).remaining()} ${d.hasRemaining()}")
