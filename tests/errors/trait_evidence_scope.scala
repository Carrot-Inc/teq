// expect: 10:23: error: no given instance of type A[Int]
// expect: 1 error found
trait A[T]:
  def f: Int
trait B[T: A]:
  def show = summon[A[T]].f
object Elsewhere:
  given a1: A[Int]:
    def f = 1
class D extends B[Int]:
  given a2: A[Int]:
    def f = 2
@main def main(): Unit = println(D().show)
