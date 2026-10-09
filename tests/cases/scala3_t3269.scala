// Adapted from scala3 tests/run/t3269.scala (Apache-2.0, see tests/scala3/README.md).
object Test {
  def main(args: Array[String]): Unit = {
    val it = List(1).iterator ++ { println("Hello"); Iterator.empty }
    println(it.next())
    it.hasNext
    it.hasNext
    it.hasNext
  }
}
