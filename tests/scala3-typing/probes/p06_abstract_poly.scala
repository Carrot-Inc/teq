trait P[A]:
  def a[T]: A
class C extends P[Int]:
  def a = 1
trait Q:
  def f(x: Int): Int
class D extends Q:
  def f(x: String): Int = 1
@main def run(): Unit = println(C().a)
