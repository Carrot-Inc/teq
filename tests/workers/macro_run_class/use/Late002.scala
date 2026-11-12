// A plain call of the macro.
package plot.use

import plot.Scaling

object Late002:
  val scaled: Int = Scaling.scaled(2)
  def sum(xs: List[Int]): Int = xs.foldLeft(scaled)(_ + _)
