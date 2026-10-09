// Adapted from scala3 tests/run/t5407.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
case class Foo(private val x: Int, y: Option[Int], z: Boolean)

object Test {
  def main(args: Array[String]): Unit = ()
  def foo(x: Foo) = x match {
    case Foo(x, Some(y), z) => y
    case Foo(x, y, z) => 0
  }
  val x = Foo(1, Some(2), false)
  println(foo(x))


  def bar(x: Foo) = x match {
	  case Foo(x, Some(y), z) => y
	  case Foo(x, None, z) => 0
	}
  println(bar(x))
}
