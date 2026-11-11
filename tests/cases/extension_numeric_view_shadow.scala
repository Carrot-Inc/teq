// An inapplicable lexical extension leaves Predef's numeric view available.
object TextOps:
  extension (text: String) def until(size: Int): String = text.take(size)
import TextOps.*
@main def run(): Unit =
  println((1 until 4).toList.map(x => x * 2))
  println("abcd".until(2))
