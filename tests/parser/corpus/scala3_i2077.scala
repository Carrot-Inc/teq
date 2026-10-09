// Adapted from scala3 tests/run/i2077.scala (Apache-2.0, see tests/scala3/README.md); replaced: Scala 2 syntax of if.
object Test {
  inline val x = true
  val y = if x then 1 else 2  // reduced to val y = 1

  def main(args: Array[String]): Unit  =
    if { println("hi"); true } then  // cannot be reduced
      1
    else
      2
}
