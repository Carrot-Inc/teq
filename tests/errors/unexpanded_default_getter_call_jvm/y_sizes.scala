package rounds

object Sizes:
  inline def two: Int = 2
  def sized(xs: Array[Int], upTo: Int = two): Int = xs.take(upTo).length
