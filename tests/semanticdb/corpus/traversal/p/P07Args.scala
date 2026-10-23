package p07
object Args:
  def f(a: Int, b: Int = 2, c: Int = 3): Int = a + b + c
  val x = f(1, c = 4)
  val y = f(b = 5, a = 1)
  def g(x: Int)(y: Int = x): Int = y
  val z = g(1)()
  class C(val p: Int = 1, q: Int = 2)
  val cc = new C(q = 3)
  def h = f(c = { val loc = 1; loc }, a = x)
  def k(n: Int) = f(c = n, a = n)
