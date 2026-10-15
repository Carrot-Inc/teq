package ndr

object Client {
  def main(args: Array[String]): Unit = {
    val o = new Outer(5)
    println(o.Item(o.state).b)
  }
}
