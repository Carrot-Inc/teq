// Adapted from scala3 tests/run/t405.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  val x = M;
  object M;
  assert(x eq M)
}
