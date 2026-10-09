package eoa

object A:
  def f(s: String): String = s
  def f(n: Int = 7): Int = n

// An overloaded name exported: the alternative with a default forwards its getter too.
object B:
  export A.f
