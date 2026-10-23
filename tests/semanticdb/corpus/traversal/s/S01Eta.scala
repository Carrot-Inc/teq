package s01
object Eta:
  def eff(i: Int): Int = i
  def f(a: Int)(b: Int): Int = a + b
  val h = List(1).map(f(eff(1)))
  val k = List((1, 2)).map((a, b) => a + b)
  val l = List((1, 2)).map { case (c, d) => c }
  val m = List(1).map(n => n)
  def t(): Unit =
    var i = 0
    while i < 3 do i += 1
