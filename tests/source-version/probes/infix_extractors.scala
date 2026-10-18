object Outer { def unapply(x: Int): Some[(Int, Int)] = Some((x, x)) }
object Inner { infix def unapply(x: Int): Some[(Int, Int)] = Some((x, x)) }
case class C(a: Int, b: Int)
object C { infix def unapply(x: Int): Some[(Int, Int)] = Some((x, x)) }
case class D(a: Int, b: Int)
infix case class E(a: Int, b: Int)
object P {
  def f(x: Int): Int = x match { case (a Inner b) Outer c => a + b + c }
  def g(x: Int): Int = x match { case a Outer (b Inner c) => a + b + c }
  def h(x: Int): Int = x match { case a C b => a + b }
  def k(d: D): Int = d match { case (a Inner b) D c => a + b + c }
  def l(e: E): Int = e match { case (a Outer b) E c => a + b + c }
}
