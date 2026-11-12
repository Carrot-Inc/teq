// The extension macro's call as an argument of a transparent inline call.
package plot.use

import plot.Scaling
import plot.Scaling.scaledUp

object Early007:
  val picked: Int = Scaling.pick(7.scaledUp)
  def sum(xs: List[Int]): Int = xs.foldLeft(picked)(_ + _)
