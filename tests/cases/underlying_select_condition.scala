class V(val n: Int) extends AnyVal

object Main:
  def measure(n: Int): V = new V(n)

  def sign(n: Int): String =
    if 0 < measure(n)
      .n then "positive" else "nonpositive"

  def main(args: Array[String]): Unit =
    println(sign(3))
    println(sign(-1))
