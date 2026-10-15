package ndo

object Client {
  def main(args: Array[String]): Unit = {
    val o = new Outer(3)
    println(o.Item().x)
    println(o.Item("ab").x)
  }
}
