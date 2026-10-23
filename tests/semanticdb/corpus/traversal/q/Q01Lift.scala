package q01
object Lift:
  def f(a: Int, b: Int = 2, c: Int = 3): Int = a + b + c
  def eff(i: Int): Int = i
  val x = f(c = eff(1), a = eff(2))
  def loc: Int =
    def g(p: Int, q: Int = 5)(r: Int = p): Int = p + q + r
    g(1)() + g(q = eff(3), p = eff(4))()
