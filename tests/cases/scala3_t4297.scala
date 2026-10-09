// Adapted from scala3 tests/run/t4297.scala (Apache-2.0, see tests/scala3/README.md).
object Test {
  def main(args: Array[String]): Unit = {
    def f = List(1,2,3).view
    assert(f.toString == "SeqView(<not computed>)")
    assert(f.mkString == "123")
  }
}
