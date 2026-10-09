// jars: refined
//> using dep eu.timepit::refined:0.11.4
// refined's `autoUnwrap` from `eu.timepit.refined.auto` applied to a receiver: `<=` on an
// `Int Refined Greater[1]`, and the expected-type conversion beside it.
import eu.timepit.refined.api.Refined
import eu.timepit.refined.auto.autoUnwrap
import eu.timepit.refined.numeric.Greater
import eu.timepit.refined.refineV

object Main:
  def atLimit(max: Int Refined Greater[1], used: Int): Boolean = max <= used
  def main(args: Array[String]): Unit =
    val max = refineV[Greater[1]](3).toOption.get
    println(atLimit(max, 2))
    println(atLimit(max, 3))
    val n: Int = max
    println(n + 1)
