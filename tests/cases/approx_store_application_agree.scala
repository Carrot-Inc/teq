// The application of a type lambda:
// a class's parameter instantiated to a lambda whose result is a compiletime
// operation (`3` through `b.f`), a lambda whose result applies a match alias over a wildcard argument (`String`),
// a given's lambda over a contravariant class, and an abstract member applied through a bounded prefix in a dependent
// result (`A#FCt[Null]`, the member's declared contravariance). scalac 3.8.4 accepts each, and teq is to accept each.
import scala.compiletime.ops.int.+
class Box[F[_ <: Int], A <: Int]:
  def f: F[A] = ???
val b = new Box[[X <: Int] =>> X + 1, 2]
def three: 3 = b.f
type M[X] = X match { case Int => String }
class Box2[F[_], A]:
  def f: F[A] = ???
def mk: Box2[[X] =>> M[X], ? <: Int] = new Box2
def str: String = mk.f
class KC[-X]
trait Has[F[_], A]:
  def f: F[A]
given Has[[X] =>> KC[X], Int] with
  def f: KC[Int] = new KC
def use[A](using h: Has[[X] =>> KC[X], A]): KC[A] = h.f
class Cov[+X]
class Am:
  type FCt[-X]
  type T >: Null <: String
def member(f: (c: Am) => Cov[c.FCt[c.T]]): Am => Cov[Am#FCt[Null]] = f
@main def approxStoreApplicationAgree(): Unit =
  println(use[Int].getClass.getSimpleName)
