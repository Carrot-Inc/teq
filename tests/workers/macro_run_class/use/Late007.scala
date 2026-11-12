// A plain call of the macro.
package plot.use

import plot.Scaling

object Late007:
  val scaled: Int = Scaling.scaled(7)
  def sum(xs: List[Int]): Int = xs.foldLeft(scaled)(_ + _)
