// Adapted from scala3 tests/run/indexedSeq-apply.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  val empty = IndexedSeq()
  assert(empty.isEmpty)

  val single = IndexedSeq(1)
  assert(List(1) == single.toList)

  val two = IndexedSeq("a", "b")
  assert("a" == two.head)
  assert("b" == two.apply(1))

  println("OK")
}

// vim: set ts=2 sw=2 et:
