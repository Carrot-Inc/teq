// A def whose result type is missing after its `:`.
object O:
  def f(x: Int): = x + 1
  val y: String = f(1)
  val bad: String = 2
