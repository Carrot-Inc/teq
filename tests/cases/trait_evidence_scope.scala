// The evidence of a trait's context bound is resolved where the extends clause stands: the
// class's own givens are not in scope there, and the constructor's using parameters are.
trait A[T]:
  def f: Int
trait B[T: A]:
  def show = summon[A[T]].f
given a1: A[Int]:
  def f = 1
class D extends B[Int]:
  given a2: A[Int]:
    def f = 2
class E(using A[Int]) extends B[Int]
object Local:
  given a3: A[Int]:
    def f = 3
  class G extends B[Int]
@main def main(): Unit =
  println(D().show)
  println(E(using a1).show)
  println(E(using D().a2).show)
  println(Local.G().show)
  println(new B[Int] { given a4: A[Int] { def f = 4 } }.show)
