object Implicit {
  def f(x: Int)(using y: Int): Int = x + y
  def g(using Int): Int = f(1)
}
