package ndn

object Main {
  def main(args: Array[String]): Unit = {
    val o = new Outer(3)
    println(o.Item().x)
    println(o.Item(x = "abcd").x)
    println(o.Exact(2).x)
    println(new o.Exact().x)
  }
}
