package probe

object Main {
  def main(args: Array[String]): Unit = {
    val g: Int => Int = new H
    println(g(2))
  }
}
