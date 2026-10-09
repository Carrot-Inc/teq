package caa

case class C(x: Int)
object C:
  def apply(s: String): C = new C(s.length)

case class D(x: Int, y: String)
object D:
  def apply(x: Int): D = new D(x, "")
