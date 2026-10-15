package nds

object Client {
  def main(args: Array[String]): Unit = {
    val o = new Outer(5)
    println(new o.Sec("abc").a)
    println(new o.Const("ab").x)
    println(new o.Curried("a")().a)
  }
}
