// The duration syntax classes imported by name are the implicit conversions scala-library's
// `implicit final class DurationLong` and its siblings are; the wildcard import keeps working.
object Named:
  import scala.concurrent.duration.DurationLong
  def run(): Unit =
    val ms: Long = 150000L
    println(ms.milli.toMinutes)
    println(ms.millis)
    println(DurationLong(3L).seconds)

object NamedInt:
  import scala.concurrent.duration.{DurationInt, DurationDouble}
  def run(): Unit =
    println(5.seconds)
    println(1.5.seconds)
    println(2.minutes.toSeconds)

object Wildcard:
  import scala.concurrent.duration.*
  def run(): Unit =
    println(1.milli)
    println(2L.hours)
    println(0.5.minutes)
    println(3.seconds + 250.millis)

object Main:
  def main(args: Array[String]): Unit =
    Named.run()
    NamedInt.run()
    Wildcard.run()
