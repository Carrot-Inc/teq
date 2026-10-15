package ndbc

object Main {
  def main(args: Array[String]): Unit = {
    val o = new Outer(5)
    val i = new o.Item()()()
    println(i.c)
    println(i.get)
    println(o.count)
  }
}
