// Adapted from scala3 tests/run/t8738.scala (Apache-2.0, see tests/scala3/README.md); replaced: Scala 2 syntax of for.
object Test {
  def check(a: Range, b: Range) = (a == b) == (a.toList == b.toList)
  def main(args: Array[String]): Unit = {
    val lo = -2 to 2
    val hi = lo
    val step = List(-6,-2,-1,1,2,6)
    for i <- lo; j <- hi; n <- step; k <- lo; l <- hi; m <- step do {
      assert(
        check(i until j by n, k until l by m) &&
        check(i until j by n, k to l by m) &&
        check(i to j by n, k until l by m) &&
        check(i to j by n, k to l by m)
      )
    }
  }
}
