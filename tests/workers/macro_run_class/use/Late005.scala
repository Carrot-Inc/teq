// A plain call of the macro.
package plot.use

import plot.Scaling

object Late005:
  val scaled: Int = Scaling.scaled(5)
  def sum(xs: List[Int]): Int = xs.foldLeft(scaled)(_ + _)
