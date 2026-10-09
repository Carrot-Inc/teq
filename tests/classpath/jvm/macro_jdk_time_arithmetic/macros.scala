// std: scala-library
// jars: scala-library
// The interpreter's native java.time members at compile time, against the JDK's: truncation
// toward zero, overflow, Instant's range and the comparison's magnitude.
import scala.quoted.*
import java.time.{Duration, Instant, DateTimeException}

object Probe:
  inline def run: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    def attempt(f: => Any): String =
      try f.toString
      catch
        case _: ArithmeticException => "overflow"
        case _: DateTimeException => "range"
    val lines = List(
      attempt(Duration.ofSeconds(-1, 999999999).toMillis),
      attempt(Duration.ofMillis(-1).toMillis),
      attempt(Duration.ofSeconds(-2, 500000000).toMillis),
      attempt(Duration.ofSeconds(-1, 1).toNanos),
      attempt(Duration.ofSeconds(9223372036854775807L).toNanos),
      attempt(Duration.ofSeconds(9223372036854775807L).toMillis),
      attempt(Instant.ofEpochSecond(9223372036854775L).toEpochMilli),
      attempt(Instant.ofEpochSecond(31556889864403200L).getEpochSecond),
      attempt(Instant.ofEpochSecond(31556889864403199L).getEpochSecond),
      attempt(Instant.ofEpochSecond(-31557014167219201L).getEpochSecond),
      attempt(Instant.ofEpochMilli(Long.MinValue).getEpochSecond),
      attempt(Instant.ofEpochSecond(0, 2).compareTo(Instant.ofEpochSecond(0, 0))),
      attempt(Duration.ofSeconds(0, 2).compareTo(Duration.ofSeconds(0, 0))),
      attempt(Instant.ofEpochSecond(5).compareTo(Instant.ofEpochSecond(0))),
      attempt(Duration.ofSeconds(-3, 250000000)),
      attempt(Instant.ofEpochSecond(-1, 500))
    )
    Expr(lines.mkString("\n"))
