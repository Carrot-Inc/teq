package ndc

object Client {
  def main(args: Array[String]): Unit = {
    val o = new Outer(10)
    val i = o.Item(1)()
    println(i.toString + " " + i.c + " " + o.made)
  }
}
