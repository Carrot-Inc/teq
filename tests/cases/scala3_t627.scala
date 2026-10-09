// Adapted from scala3 tests/run/t627.scala (Apache-2.0, see tests/scala3/README.md).
object Test {
  def main(args: Array[String]): Unit = {
    val s: Seq[Int] = Array(1, 2, 3, 4).toIndexedSeq
    println(s)
  }
}
