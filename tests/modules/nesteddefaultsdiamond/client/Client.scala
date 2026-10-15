package ndd

object Client {
  def main(args: Array[String]): Unit = {
    val t: T = new Impl
    println(new t.Item().x)
    println(t.Item(7).x)
  }
}
