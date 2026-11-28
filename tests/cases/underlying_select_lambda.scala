class V(val n: Int) extends AnyVal

object Main:
  def make(n: Int): V = new V(n)

  def main(args: Array[String]): Unit =
    val add: Int => Int = x => x + make(x)
      .n
    println(add(4))
