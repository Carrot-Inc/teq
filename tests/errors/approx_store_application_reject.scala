// The application of a type lambda and a match
// alias's approximation where scalac 3.8.4 rejects
// and teq is to reject: scalac captures the wildcard
// (`h.KC[?1.A]`), folds `2 + 1` to `(3 : Int)`, and
// approximates `Contra[MT[c.T]]` to `Contra[Nothing]`.
import scala.compiletime.ops.int.+
class H:
  type KC[-X]
val h = new H
class Box[F[_], A]:
  def f: F[A] = ???
def mk: Box[[X] =>> h.KC[X], ? <: Int] = new Box
def contra: h.KC[Int] = mk.f
class Box2[F[_ <: Int], A <: Int]:
  def f: F[A] = ???
val b = new Box2[[X <: Int] =>> X + 1, 2]
def folded: String = b.f
class Contra[-X]
type MT[X] = Int match { case X => String }
class A:
  type T
def alias(f: (c: A) => Contra[MT[c.T]]): Int = f.andThen(x => x)
@main def main(): Unit = ()
// expect: 13:28: error: Found:    h.KC[?1.A]
// expect: 17:24: error: Found:    (3 : Int)
// expect: 22:57: error: Found:    A => Contra[Nothing]
// expect: Required: h.KC[Int]
// expect: Required: String
// expect: Required: Int
