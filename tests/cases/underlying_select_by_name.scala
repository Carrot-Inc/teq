class V(val n: Int) extends AnyVal

object Main:
  def take(n: => Int): Unit = println(n)
  def make(): V =
    println("make")
    new V(6)

  def main(args: Array[String]): Unit =
    take(1 + make()
      .n)
