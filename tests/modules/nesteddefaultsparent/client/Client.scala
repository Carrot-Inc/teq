package ndpa

object Client {
  def main(args: Array[String]): Unit = {
    val o = new Outer
    def prefix(): Outer = { println("receiver"); o }
    val i = prefix().Item({ println("argument"); o.state })
    println(i.b)
    println(o.Mixed(2).b)
  }
}
