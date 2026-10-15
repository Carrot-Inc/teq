package ndp

object Main {
  def main(args: Array[String]): Unit = {
    val o = new Outer(3)
    val k = new o.K()()
    println(k.x.toString + k.y + k.z)
    println(new o.K(y = "e")(1).y)
    println(new o.W().w.toString + o.W.tag)
  }
}
