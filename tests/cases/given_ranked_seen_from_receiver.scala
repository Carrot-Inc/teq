// Two givens one of which a receiver specializes are ranked by their reference types (dotty's
// `Applications.compare` over `alt.widen`, 2367-2373): `O.value` of `G[T]`'s `value: T` is an `A`
// through `object O extends G[A]`, more general than the local `b: B`, which Scala 3.7's
// preference takes; through `P extends G[B]` the two are alike and the owners decide nothing.
trait Base
class A extends Base
class B extends A
trait G[T <: Base]:
  def make: T
  given value: T = make
object O extends G[A]:
  def make = new A
object P extends G[B]:
  def make = new B
@main def run(): Unit =
  locally {
    import O.given
    given b: B = new B
    println(summon[A].isInstanceOf[B])
  }
  locally {
    import O.given
    import P.given
    println(summon[B].isInstanceOf[B])
  }
