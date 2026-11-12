// The extension macro's call as an argument of a transparent inline call.
package plot.use

import plot.Scaling
import plot.Scaling.scaledUp

object Early003:
  val picked: Int = Scaling.pick(3.scaledUp)
  def sum(xs: List[Int]): Int = xs.foldLeft(picked)(_ + _)
