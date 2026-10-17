class C(x: Int)(implicit y: Int)
class D extends C(1)(2)
class E extends C(1) /* ( */ (2)
object P {
  infix def f(implicit x: Int): Int = x
  infix def g(implicit x: Int, y: Int): Int = x
  def h(a: Int)(implicit b: Int): Int = b
  val a = P f /* ( */ 1
  val b = P g (1, 2)
  val n = new C(1)(2)
  val q = h(1) /* ( */ { 2 }
  val r = h(1) { 2 }
}
