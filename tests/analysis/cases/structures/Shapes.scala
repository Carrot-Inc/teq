package st

trait A
trait B
trait C

// The structures of intersections and unions, one per type as scalac's caches keep them.
class Shapes:
  def f(x: A & B): A | B = x
  def g(x: A & B, y: A & B): A & B = x
  def h(x: A | B, y: B | A): Unit = ()
  def l(x: (A & B) | C): Unit = ()
  def o(x: A & B): Unit = ()
  def o(x: A & B, y: Int): Unit = ()
  def p[X](x: X & A): Unit = ()
  def p[X](x: X & A, y: Int): Unit = ()
  def q[X](x: X & A): Unit = ()
  def r[X](x: X & A): Unit = ()

trait Show:
  def show(x: A | B): String
  def both(x: A & B, y: A | B): Unit
