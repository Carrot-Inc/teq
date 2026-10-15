package ndb2

object Client {
  def main(args: Array[String]): Unit = {
    val o = new Outer(5)
    println(new o.Item()().b)
    println(o.Pair(1)()().z)
    println(o.reads)
  }
}
