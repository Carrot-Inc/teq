// The extension macro's call as an argument of a transparent inline call.
package plot.use

import plot.Scaling
import plot.Scaling.scaledUp

object Early04:
  def b0(x: Int): Int = List(x, 0).map(_ + 1).foldLeft(0)(_ + _)
  def b1(x: Int): Int = List(x, 1).map(_ + 1).foldLeft(0)(_ + _)
  def b2(x: Int): Int = List(x, 2).map(_ + 1).foldLeft(0)(_ + _)
  def b3(x: Int): Int = List(x, 3).map(_ + 1).foldLeft(0)(_ + _)
  def b4(x: Int): Int = List(x, 4).map(_ + 1).foldLeft(0)(_ + _)
  def b5(x: Int): Int = List(x, 5).map(_ + 1).foldLeft(0)(_ + _)
  def b6(x: Int): Int = List(x, 6).map(_ + 1).foldLeft(0)(_ + _)
  def b7(x: Int): Int = List(x, 7).map(_ + 1).foldLeft(0)(_ + _)
  val picked: Int = Scaling.pick(4.scaledUp)
  def sum(xs: List[Int]): Int = xs.foldLeft(picked)(_ + _)
