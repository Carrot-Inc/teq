package ndo

object Main {
  def main(args: Array[String]): Unit = {
    val o = new Outer(3)
    println(o.Item().x)
    println(o.Item(7).x)
    println(o.Item("abcd").x)
  }
}
