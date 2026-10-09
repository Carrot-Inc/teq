// Adapted from scala3 tests/neg/union.scala (Apache-2.0, see tests/scala3/README.md); replaced: class inheritance by traits.
// expect: type mismatch: found A, required B | C
// expect: type mismatch: found Top, required A | B
object Test:
  trait A
  class B extends A
  class C extends A
  class D extends A

  val b = true
  val x = if b then B() else C()
  val y: B | C = x  // error

object O:
  trait Top
  class A extends Top
  class B extends Top
  def f[T](x: T, y: T): T = x

  val x: A = f(A(), A())

  val y1: A | B = f(A(), B()) // ok
  val y2: A | B = f[A | B](A(), B()) // ok

  val z = if ??? then A() else B()

  val z1: A | B = z // error

  val z2: A | B = if ??? then A() else B() // ok
