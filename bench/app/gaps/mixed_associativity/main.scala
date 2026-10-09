// scalac rejects `a :: List(b) :+ c`: `::` is right-associative and `:+` left-associative at
// the same precedence; teq accepts it.
object Main:
  def main(args: Array[String]): Unit =
    val xs = 1 :: List(2) :+ 3
    println(xs)
