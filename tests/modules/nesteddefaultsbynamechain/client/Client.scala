package ndbc

object Client {
  def main(args: Array[String]): Unit = {
    val o = new Outer(1)
    println(new o.Item(10)()().c)
    println(o.count)
  }
}
