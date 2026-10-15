package ndp

object Client {
  def main(args: Array[String]): Unit = {
    val o = new Outer(3)
    val k = new o.K(5)()
    println(k.x.toString + k.y + k.z)
    println(new o.W().w)
  }
}
