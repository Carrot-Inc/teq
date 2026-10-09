package sba

// An expansion's binding `x` beside a local `x` of the site that the expansion still reads.
object F:
  transparent inline def add(x: Int, inline y: Int): Int = x + y

object Use:
  def run(n: Int): Int =
    val x = n + 100
    F.add(n + 1, x)
