// A variable an application's result holds only under an abstract type constructor declared
// covariant (`type Type[+A]`, the shape of cats' `NonEmptyChain`) is covariant there, as dotty's
// `Inferencing.variances` reads the constructor's parameter variances: instantiated from below,
// `Nothing`, so the value conforms to any `V[Impl.Type[E], Int]`, as under scalac.
object Impl:
  type Base
  trait Tag
  type Type[+A] <: Base & Tag
class V[+E, +A]
def valid[E, A](a: A): V[Impl.Type[E], A] = new V
def run(): Unit =
  val y = valid(1)
  val z: V[Impl.Type[String], Int] = y
  val t: Int = y
// expect: 14:16: error: type mismatch: found V[Type[Nothing], Int], required Int
// absent: 13:38:
