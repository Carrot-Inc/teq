package ndtp

object Client {
  def main(args: Array[String]): Unit = {
    val t: T = new Impl
    println(new t.Item()().y)
  }
}
