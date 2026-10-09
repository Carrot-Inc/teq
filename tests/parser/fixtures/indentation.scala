// A line between two indentation widths stays in the inner region.
object O:
  def f(x: Int): Int =
    val a = x + 1
   val b = a + 1
    b
  def g: String = 1
object P:
  val v: String = 2
