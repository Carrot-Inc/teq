// A def whose `=` is missing on its line: the expression after the header is its body.
object O:
  def f(x: Int): Int (x + 1) * 2
  def g(y: Int): Int = f(y) + 1
  val bad: String = 2
