package plot

trait Step:
  def apply(n: Int): Int
object Deep:
  def inner(n: Int) =
    val bias = n * 2
    val s = new Step { def apply(m: Int): Int = m + bias + 1 }
    s(n)
