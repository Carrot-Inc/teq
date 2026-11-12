// A plain call of the macro.
package plot.use

import plot.Scaling

object Late001:
  val scaled: Int = Scaling.scaled(1)
  def sum(xs: List[Int]): Int = xs.foldLeft(scaled)(_ + _)
