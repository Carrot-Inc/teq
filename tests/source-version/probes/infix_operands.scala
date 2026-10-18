class C {
  def f(x: Int): Int = x
  def g(x: Int)(y: Int): Int = x + y
  infix def h(x: Int)(y: Int): Int = x + y
  transparent inline def t(x: Int): Int = x + 1
  inline def u(x: Int): Int = x + 1
  infix transparent inline def v(x: Int): Int = x + 1
}
object P {
  val c = new C
  val a = c f ({ val a = 1; a })
  val b = c f { val a = 1; a }
  val d = c f ({ 2 })
  val e = c f { 2 }
  val k = c g 1
  val l = c h 1
  val m: Int => Int = c g 2
  val n = c f
    3
  val p = c t 1
  val q = c u 1
  val r = c v 1
}
