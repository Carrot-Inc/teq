// A def whose `:` before the result type is missing: the header skips to its `=`.
object O:
  def f(x: Int) Int = x + 1
  def g(y: Int): Int = f(y) + 1
  val bad: String = 2
