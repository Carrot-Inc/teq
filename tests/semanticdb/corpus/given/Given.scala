object Given {
  given Int = 1
  def f(using Int): Int = summon[Int]
  val x = f
}
