// `Duration` as scala-library has it: an abstract class with `FiniteDuration` and the three
// infinite values as its subclasses, overloads over both telling the finite result apart.
import scala.concurrent.duration.*

object Main:
  def sleep(d: Duration): String = "duration " + d
  def sleep(d: FiniteDuration): String = "finite " + d

  def describe(d: Duration): String = d match
    case f: FiniteDuration => "finite " + f.toMillis
    case _ => "infinite " + d

  def main(args: Array[String]): Unit =
    println(Duration.Inf.toString + " " + Duration.MinusInf + " " + Duration.Undefined)
    println(describe(5.seconds) + " " + describe(Duration.Inf))
    println(Duration(1.5, SECONDS))
    println(Duration(Double.PositiveInfinity, SECONDS) eq Duration.Inf)
    println(Duration.Inf > 5.seconds)
    println(s"${Duration.MinusInf < Duration.Zero} ${Duration.Inf.isFinite} ${-Duration.Inf}")
    val sum: FiniteDuration = 5.seconds + 3.seconds
    println(sum)
    val mixed: Duration = 5.seconds + Duration.Inf
    println(mixed)
    println(sleep(2.seconds) + ", " + sleep(Duration.Inf))
    println(s"${Duration.Inf + Duration.MinusInf} ${Duration.Inf * 0.0} ${Duration.Undefined == Duration.Undefined}")
    println(List[Duration](Duration.Inf, 3.seconds, Duration.MinusInf, 1.second).sorted)
    println(10.seconds / 4.seconds)
    println(s"${2.seconds * 1.5} ${Duration.Inf / 2.0}")
    println(s"${5.seconds.toCoarsest} ${120.seconds.toCoarsest}")
    println(s"${Duration.Inf.min(3.seconds)} ${(3.seconds: Duration).max(Duration.Inf)}")
    println(scala.util.Try(Duration.Inf.toMillis).isFailure)
    val Duration(n, u) = 3.minutes: @unchecked
    println(n.toString + " " + u)
