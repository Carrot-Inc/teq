object P {
  def f(implicit x: Int): Int = x
  val a = f:
    (1)
  val b = f:
    { 2 }
  val c = f:
    val x = 1
    x
}
