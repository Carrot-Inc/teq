import java.nio.ByteBuffer
import java.nio.charset.StandardCharsets

// What `Charset.decode` gives is a buffer of its own, which takes writes.
@main def main(): Unit =
  val b = StandardCharsets.UTF_8.decode(ByteBuffer.wrap(Array[Byte](97, 98)))
  println(s"${b.isReadOnly()} ${b.position()} ${b.limit()}")
  b.put('x')
  println(s"${b.position()} ${b.get()}")
  b.flip()
  println(b.toString)
