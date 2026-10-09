// Adapted from scala3 tests/run/i5606.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()

  extension [A, B](f: A => B) def `$` (a: A): B = f(a)

  assert((((a: Int) => a.toString()) `$` 10) == "10")

  def g(x: Int): String = x.toString

  assert((g `$` 10) == "10")

  val h: Int => String = _.toString

  assert((h `$` 10) == "10")
}
