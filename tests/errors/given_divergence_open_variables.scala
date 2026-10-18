// expect: 30:38: error: no given instance of type Show[W[P[Int, Long]]] was found for parameter x
// expect: 31:26: error: no given instance of type Show[Int] was found for parameter x
// expect: 32:13: error: no given instance of type Show[P[
// expect: 3 errors found
// Recursive givens without a base instance whose requests hold open type variables (a using clause's `Out`):
// the swap alternates between two requests of one size, the growth asks for larger ones of the same classes, and
// a request under a bound that names its variable (`A <: Ranked[A]`) reaches both; every search ends by the
// divergence check and is refused as scalac refuses it.
trait Show[A]
final case class P[A, B](a: A, b: B)
final case class W[A](a: A)

trait Rot[T] { type Out }
object Rot:
  type Aux[T, O] = Rot[T] { type Out = O }
  given swap[A, B]: Aux[P[A, B], P[B, A]] = new Rot[P[A, B]] { type Out = P[B, A] }
given rotated[T, O](using r: Rot.Aux[T, O], s: Show[W[O]]): Show[W[T]] = new Show[W[T]] {}

trait Wrap[T] { type Out }
object Wrap:
  type Aux[T, O] = Wrap[T] { type Out = O }
  given opt[T]: Aux[T, Option[T]] = new Wrap[T] { type Out = Option[T] }
given grow[T, O](using w: Wrap.Aux[T, O], s: Show[O]): Show[W[T]] = new Show[W[T]] {}
given unwrap[T](using s: Show[W[T]]): Show[T] = new Show[T] {}

trait Ranked[A <: Ranked[A]]
given nest[T](using s: Show[T]): Show[P[T, T]] = new Show[P[T, T]] {}
def best[A <: Ranked[A]](using s: Show[P[A, A]]): Unit = ()

def a = summon[Show[W[P[Int, Long]]]]
def b = summon[Show[Int]]
def c = best
