// A given whose type is an abstract type member of its owner (`value: T` of `G { type T }`) is
// specialized by the receiver it is read on, as a type parameter is: through `O`, which fixes
// `T = A`, it is an `A`, eligible for an `A` and ranked by that type (`Applications.compare` over
// `alt.widen`), the more general one beside a local `given b: B`, as scalac selects it.
trait Base
class A extends Base
class B extends A
trait G:
  type T <: Base
  def make: T
  given value: T = make
object O extends G:
  type T = A
  def make = new A
@main def run(): Unit =
  import O.given
  given b: B = new B
  println(summon[A].isInstanceOf[B])
  println(summon[O.T].isInstanceOf[B])
