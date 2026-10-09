// Adapted from scala3 tests/pos/i11993.scala (Apache-2.0, see tests/scala3/README.md): a class
// parameter typed by an earlier one (`val y: Option[x.type]`), for a class and a trait; the
// prints are added.
object test1:
  class Foo(
    val x: String,
    val y: Option[x.type]
  )
  class Bar extends Foo("bar", Some("bar"))

object test2:
  trait Foo(
    val x: String,
    val y: Option[x.type]
  )
  class Bar extends Foo("bar", Some("bar"))

@main def main(): Unit =
  val b = test1.Bar()
  println(b.y.map(_ + b.x))
  val c = test2.Bar()
  val same: c.x.type = c.x
  println(c.y.contains(same))
