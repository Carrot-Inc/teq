object P {
  def f(): Int = 1
  def g(x: Int): Int = x
  def h(x: Int)(y: Int): Int = x + y
  def v(xs: Int*): Int = xs.size
  def n: Int = 3
  val k: Int => Int = x => x
  val a = f _
  val b = g _
  val c = h _
  val d = h(1) _
  val e = v _
  val m = k _
  val x = 1
  def main(args: Array[String]): Unit = println((a(), b(2), c(1)(2), d(3), e(Seq(1, 2)), m(4)))
}
