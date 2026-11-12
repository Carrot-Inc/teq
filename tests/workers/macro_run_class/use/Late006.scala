// A plain call of the macro.
package plot.use

import plot.Scaling

object Late006:
  val scaled: Int = Scaling.scaled(6)
  def sum(xs: List[Int]): Int = xs.foldLeft(scaled)(_ + _)
