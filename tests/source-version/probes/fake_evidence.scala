object P {
  def f(implicit evidence$1: Int): Int = evidence$1
  val x = f(1)
}
