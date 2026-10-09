// Adapted from scala3 tests/run/t7985.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  Array(1) match { case _: Array[scala.Int] => }
}
