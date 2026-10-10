object Lib:
  @deprecated("use other", "2.0") def m(x: Int): Int = x
  @deprecated def n: Int = 1
  @deprecated("gone") val v: Int = 2
  @deprecated("", "3.1") var w: Int = 3

@deprecated("old class", "1.0") class Old:
  def f = 1

@deprecated("old object", "1.1") object Gone:
  val g = 2

class Uses:
  def a = Lib.m(1) + Lib.n + Lib.v + Lib.w
  def b = new Old().f
  def c = Gone.g
  def d(o: Old): Int = o.f
  @deprecated("too") def e = Lib.m(2)

@deprecated("all of it", "4") class Inside:
  def h = Lib.m(3)

@main def run(): Unit = println(Uses().a + Uses().b + Uses().c)
