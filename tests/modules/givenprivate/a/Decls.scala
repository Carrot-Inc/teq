package gpa

import scala.compiletime.deferred

// A `private[gpa]` deferred given: its implementation keeps the boundary in the products (the namer
// notes it on a source given as on a val or a def), so a client outside the package is refused over
// them as over the sources, by teq and by scalac.
trait T:
  private[gpa] given x: Int = deferred
  def read: Int = x

class C(using n: Int) extends T

object Inside:
  def both: Int = new C(using 17).x + new C(using 17).read
