// Adapted from scala3 tests/run/t1373.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
// Testing whether case class params come back in the right order.
object Test {
  def main(args: Array[String]): Unit = ()
  case class Foo(private val a: String, b: String, private val c: String, d: String, private val e: String)
  val x = Foo("a", "b", "c", "d", "e")
  assert(x.toString == """Foo(a,b,c,d,e)""")
}
