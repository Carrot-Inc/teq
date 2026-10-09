// scala-library's names in `scala.concurrent.duration` that a library's bodies bind to: the
// conversion classes, the unit values, `Duration(length, unit)` and `FiniteDuration.unit`.
package durationnames

import scala.concurrent.duration.*

object Main:
  def main(args: Array[String]): Unit =
    val d = DurationInt(3).minutes
    println(d.toString + " " + d.unit + " " + d.length + " " + Duration(1500L, MILLISECONDS) + " " + DurationLong(2L).hours.toMinutes)
    println(DurationDouble(2.5).seconds.toString + " " + (IntMult(2) * 3.seconds) + " " + FiniteDuration(2L, HOURS).toUnit(MINUTES).toInt + " " + Duration(0.5, SECONDS))
    println(DurationInt(1).milli.toString + " " + DurationInt(90).seconds.toMinutes + " " + (SECONDS: TimeUnit) + " " + pairIntToDuration((5, SECONDS)) + " " + durationToPair(DurationInt(2).days))
