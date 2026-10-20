package probe
object Lib:
  inline def inc(x: Int): Int = x + 1
  transparent inline def wrap(x: Int): Int = inc(x)
