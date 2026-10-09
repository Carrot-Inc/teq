package ila

object Macros:
  inline def twice(x: Int): Int = x * 2
  transparent inline def pick(inline b: Boolean): Any = inline if b then 1 else "one"
  def plain(x: Int): Int = x + 1

class Tuner:
  inline def adjust(x: Int): Int = x - 1
  def run(x: Int): Int = adjust(x) + Macros.twice(x)
