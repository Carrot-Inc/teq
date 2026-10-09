// A string literal left open at its line's end.
object O:
  def f(s: String): Int = s.length
  val a = f("abc)
  val b: Int = f("x")
  val bad: String = 3
