object Main {
  def pair(): (Int, Int) = { println("pair"); (2, 5) }
  def sum(): Int = {
    val total = {
      val (a, b) = pair()
      val Some(x) = Some(b)
      a + x
    }
    total
  }
  def main(args: Array[String]): Unit = println(sum())
}
