// Adapted from scala3 tests/run/rawstrings.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  println(raw"[\n\t'${'"'}$$\n]")
}
