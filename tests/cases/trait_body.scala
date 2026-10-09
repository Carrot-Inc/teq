trait A[T]:
  def f: T
trait B[T: A]:
  println("  B body: " + summon[A[T]].f)
  def g: String = "g"
trait Plain:
  println("  Plain body")
  def h = 1
given a1: A[Int]:
  def f = 1
class D extends B[Int] with Plain
object O extends Plain
@main def main(): Unit =
  println("before D")
  val d = D()
  println(d.g)
  println("before O")
  println(O.h)
