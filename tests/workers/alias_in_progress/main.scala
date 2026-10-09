// Sixteen bodies, in two files so that two workers each hold one, read an abstract type member
// of a std trait that nothing typed before the fork completes: under TEQ_ALIAS_ORDER one worker
// completes it while the others meet it in progress.
import scala.deriving.Mirror

object A0:
  def f(m: Mirror.Of[Int]): Int = bound[m.MirroredElemLabels] + 0
object A1:
  def f(m: Mirror.Of[Int]): Int = bound[m.MirroredElemLabels] + 1
object A2:
  def f(m: Mirror.Of[Int]): Int = bound[m.MirroredElemLabels] + 2
object A3:
  def f(m: Mirror.Of[Int]): Int = bound[m.MirroredElemLabels] + 3
object A4:
  def f(m: Mirror.Of[Int]): Int = bound[m.MirroredElemLabels] + 4
object A5:
  def f(m: Mirror.Of[Int]): Int = bound[m.MirroredElemLabels] + 5
object A6:
  def f(m: Mirror.Of[Int]): Int = bound[m.MirroredElemLabels] + 6
object A7:
  def f(m: Mirror.Of[Int]): Int = bound[m.MirroredElemLabels] + 7

@main def main(): Unit =
  println(all)
