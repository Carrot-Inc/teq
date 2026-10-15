package ndr

object Main {
  def main(args: Array[String]): Unit = {
    val o = new Outer(5)
    def recv = { println("receiver"); o }
    def arg = { println("argument"); o.state }
    println(recv.Item(arg).b)
    val p = new Outer(6)
    println(p.Item(p.state).a)
  }
}
