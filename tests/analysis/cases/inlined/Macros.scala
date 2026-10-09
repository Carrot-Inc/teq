package ila2

object Macros:
  inline def twice(x: Int): Int = x * 2
  inline def describe(inline s: String): String = "<" + s + ">"
  transparent inline def pick(inline b: Boolean): Any = inline if b then 1 else "one"
  inline def total(xs: Int*): Int = xs.sum
