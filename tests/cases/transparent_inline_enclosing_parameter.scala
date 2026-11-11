// wildApprox preserves named references to the enclosing method's parameters.
trait Eq[A]:
  def equal(x: A, y: A): Boolean
final class EqImpl[A] extends Eq[A]:
  def equal(x: A, y: A): Boolean = x == y
object Eq:
  transparent inline def make[A]: Eq[A] = new EqImpl[A]
final case class Pair[A](first: A, second: A)
def pairEq[T]: Eq[Pair[T]] = Eq.make
def boundedEq[T <: CharSequence]: Eq[Pair[T]] = Eq.make
@main def run(): Unit =
  println(pairEq[Int].equal(Pair(1, 2), Pair(1, 2)))
  println(pairEq[String].equal(Pair("a", "b"), Pair("a", "c")))
  println(boundedEq[String].equal(Pair("x", "y"), Pair("x", "y")))
