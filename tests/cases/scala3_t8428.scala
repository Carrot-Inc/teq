// Adapted from scala3 tests/run/t8428.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  val xs = List.tabulate(4)(List(_))
  val i = xs.map(_.iterator).reduce { (a,b) =>
    a.hasNext
    a ++ b
  }

  val r1 = i.toList
  val r2 = xs.flatten.toList

  assert(r1 == r2, r1)
}
