package fp
object S:
  def outer(): Int =
    def f(x: Int): Int =
      val x = 3
      if x > 0 then f(x = x - 1) else 0
    f(x = 1)
