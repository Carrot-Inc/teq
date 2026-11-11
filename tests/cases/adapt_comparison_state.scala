// A failed value comparison restores tentative bounds before conversion inference.
import scala.language.implicitConversions

class Pair[+A, +B](val first: A, val second: B)

given [A]: Conversion[Pair[A, String], Pair[A, Int]] with
  def apply(pair: Pair[A, String]): Pair[A, Int] =
    new Pair(pair.first, pair.second.length)

def consume[A](pair: Pair[A, Int]): A = pair.first

@main def main(): Unit =
  val input = new Pair("kept", "length")
  val result = consume(input)
  val checked: String = result
  println(checked)
  val single: "literal" = "literal"
  println(single)
