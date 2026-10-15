package ndpa

object Main {
  def main(args: Array[String]): Unit = {
    val o = new Outer
    def prefix(): Outer = { println("receiver"); o }
    val i = prefix().Item({ println("argument"); o.state })
    println(i.a)
    println(i.b)
    val p = new Outer
    def other(): Outer = { println("receiver"); p }
    val m = other().Mixed({ println("argument"); p.state })
    println(m.a)
    println(m.b)
  }
}
