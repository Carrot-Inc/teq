trait Q:
  def f(x: Int): Int
  def g: Int
class D extends Q:
  def f(x: String): Int = x.length
  def g: String = "str"
@main def run(): Unit =
  val q: Q = D()
  println(q.f(12345))
  val n: Int = q.g
  println(n + 1)
