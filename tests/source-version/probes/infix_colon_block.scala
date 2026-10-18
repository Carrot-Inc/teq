class C {
  def f(x: Int): Int = x
  def g(h: Int => Int): Int = h(1)
}
object P {
  val c = new C
  val a = c f:
    val v = 1
    v
  val b = c f:
    2
  val d = c f:
    (3)
  val e = c f { (4) }
  val k = c g { case x => x + 1 }
  val l = c g ({ case x => x + 2 })
  val m = c f (5)
}
