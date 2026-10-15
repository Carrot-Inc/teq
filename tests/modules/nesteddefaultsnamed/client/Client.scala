package ndn

object Client {
  def main(args: Array[String]): Unit = {
    val o = new Outer(5)
    println(o.Item(x = 7).x)
    println(o.Exact(1).x)
    println(new o.Exact().x)
  }
}
