package ndc

object Main {
  def main(args: Array[String]): Unit = {
    val o = new Outer(10)
    def recv = { println("receiver"); o }
    def one = { println("one"); 1 }
    val i = recv.Item(one)()
    println(i.toString + " " + i.c)
    val j = new o.Item(2)(3)
    println(j.toString + " " + j.c + " " + o.made + " " + o.Item.tag)
    println(o.Item.first.c)
    println(o.Item(4, 5)().c)
  }
}
