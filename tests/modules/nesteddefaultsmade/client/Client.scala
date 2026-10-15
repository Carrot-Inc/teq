package ndm

object Client {
  def main(args: Array[String]): Unit = {
    val o = new Outer
    println(o.Item().hidden)
    println(o.Item(true) match { case o.Item(h) => h })
  }
}
