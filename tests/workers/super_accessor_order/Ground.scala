// The members a trait's `super` calls run, and what the calls are passed to.
trait Ground:
  def a(): Int = 1
  def b(): Int = 2
  def c(): Int = 3
  def n(x: Int, y: Int): Int = x - y
  def opt(x: Int = 1): Int = x
  def origin(): Pt = new Pt(1)

class Pt(val x: Int):
  inline def +(o: Pt): Pt = new Pt(x + o.x)

def pair(x: Int, y: Int): Int = x - y
inline def ignore(inline x: Int): Int = 0
inline def swapped(inline x: Int, inline y: Int): Int = pair(y, x)
