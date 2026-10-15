package ndtp

object Main {
  def main(args: Array[String]): Unit = {
    val t: T = new T {}
    println(new t.Item()().x)
    val i = new Impl
    println(new i.Item(1)().y)
  }
}
