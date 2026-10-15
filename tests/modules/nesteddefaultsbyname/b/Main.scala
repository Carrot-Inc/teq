package ndb2

object Main {
  def main(args: Array[String]): Unit = {
    val o = new Outer(5)
    println(new o.Item()().b)
    println(o.Item()().b)
    println(o.Pair(1)()().z)
    println(o.reads)
  }
}
