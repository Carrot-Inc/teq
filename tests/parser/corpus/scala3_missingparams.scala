// Adapted from scala3 tests/run/missingparams.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App, Scala 2 syntax of if.
/** Tests the optimiser. */

final class Foo(val x: Int) {
  def filter(p: Int => Boolean) =
    if p(x) then Some(x) else None

  // test that the closure elimination is not wrongly replacing
  // 'that' by 'this'
  def intersect(that: Foo) =
    filter { dummy =>
//      x // dummy
      that.x > 0
    }
}

object Test {
  def main(args: Array[String]): Unit = ()
  val foo1 = new Foo(42)
  val foo2 = new Foo(-42)

  println(foo1 intersect foo2)
}
