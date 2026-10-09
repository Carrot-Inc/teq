// Adapted from scala3 tests/run/numeric-range.scala (Apache-2.0, see tests/scala3/README.md); replaced: Scala 2 syntax of for.
object Test {
  def main(args: Array[String]): Unit = {
    val r = 'a' to 'z'
    for i <- -2 to (r.length + 2) do {
      assert(r.take(i) == r.toList.take(i), (i, r.take(i)))
      assert(r.drop(i) == r.toList.drop(i), (i, r.drop(i)))
    }
  }
}
