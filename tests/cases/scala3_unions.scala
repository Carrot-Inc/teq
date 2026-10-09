// Adapted from scala3 tests/pos/unions.scala and tests/pos/widen-union.scala (Apache-2.0, see tests/scala3/README.md); Test2 with a transparent class left out.
object Test:
  def test =
    val x = if ??? then "" else 1
    val y: String | Int = x

object Test3:
  trait A
  class B extends A
  class C extends A
  val x = if ??? then B() else C()
  val y: A = x

object Test1:
  val x: Int | String = 1
  val y = x
  val z: Int | String = y

object Test2:
  val x: 3 | "a" = 3
  val y = x
  val z: Int | String = y

object Test4:
  def g[X](x: X | String): Int = ???
  def y: Boolean | String = ???
  g[Boolean](y)
  g(y)
  g[Boolean](identity(y))
  g(identity(y))

@main def main(): Unit = println("ok")
