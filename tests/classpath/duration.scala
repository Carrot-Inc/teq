// jars: scala-library
// Checks with no error: `DurationConversions` gives `5.seconds` and, through the path-dependent
// result `ev.R` of `nanoseconds(c: C)(using ev: Classifier[C])`, `5.nanoseconds(span)` as a
// `FiniteDuration` and `3.seconds(fromNow)` as a `Deadline`.
import scala.concurrent.duration._
object Main:
  def main(args: Array[String]): Unit =
    val d: FiniteDuration = 5.seconds
    val n = 5.nanoseconds(span)
    val f: FiniteDuration = n
    val dl: Deadline = 3.seconds(fromNow)
    println(d.toMillis + f.toNanos + dl.timeLeft.toMillis)
