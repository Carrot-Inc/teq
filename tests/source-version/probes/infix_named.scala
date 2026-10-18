object P {
  infix def f(a: Int, b: Int = 0): Int = a + b
  def main(args: Array[String]): Unit = {
    println(P f (a = 1, b = 2))
    println(P f (a = 5))
  }
}
