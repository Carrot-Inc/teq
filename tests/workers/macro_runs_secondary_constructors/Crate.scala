package crates

class Crate(val size: Int):
  val base: Int = size * 2
  def this(a: Int, b: Int) = this(a + b)
  def this(s: String) =
    this(s.length, 1)
  def this(s: String, more: Boolean, by: Int = 3) = this(if more then s.length + by else s.length)

object Dock:
  def total(n: Int): String =
    val d = new Crate(n + 1)
    val e = new Crate(n, 2)
    val f = new Crate("abc")
    val g = new Crate("abcd", n % 2 == 0)
    val h = new Crate("ab", true, n)
    s"${d.base},${e.base},${f.base},${g.base},${h.base}"
