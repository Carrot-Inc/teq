import java.nio.CharBuffer
import java.nio.charset.StandardCharsets
import Macros.*

@main def run(): Unit =
  println(encoded("a b/ü"))
  println(Coding.encode("x y", StandardCharsets.UTF_8))
  println(Coding.decode("x%20y%C3%BC", StandardCharsets.UTF_8))
  val cb = CharBuffer.wrap("hello world")
  println(cb.subSequence(6, 11).toString + " " + cb.remaining() + " " + cb.position())
  println(s"${"abcdef".regionMatches(0, "abx", 0, 2)} ${"abcdef".regionMatches(4, "ef", 0, 3)}")
