package ndd

object Main {
  def main(args: Array[String]): Unit = {
    val t: T = new Impl
    println(t.Item().x)
    println(new t.Item().x)
  }
}
