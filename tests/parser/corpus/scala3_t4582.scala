// Adapted from scala3 tests/run/t4582.scala (Apache-2.0, see tests/scala3/README.md).
object Test {
  def main(args: Array[String]): Unit = {
    var flag = true
    def it = {
        flag = false
        Iterator(2)
    }
    val flat = (Iterator(Iterator(1)) ++ Iterator(it)).flatten
    assert(flag)
  }
}
