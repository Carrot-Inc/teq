// Adapted from scala3 tests/run/t5614.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  val str = s"a\nb"
  println(str.length)
  println(str)
}
