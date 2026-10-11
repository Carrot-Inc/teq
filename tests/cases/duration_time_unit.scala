// `duration.SECONDS`, which scala-library pickles as `TimeUnit.SECONDS.type`, is a path: the std's object standing
// for `TimeUnit`'s statics declares the constant as a def of no clauses, a static field, stable as dotty's
// `ClassfileParser` reads a final one (master, on the JVM: "not supported yet: singleton type", the overload of
// `Duration.apply(Double, TimeUnit)` not found).
import scala.concurrent.duration.*
import java.util.concurrent.TimeUnit
@main def run() =
  println(Duration(1.5, SECONDS))
  println(Duration(2, MILLISECONDS))
  println(FiniteDuration(3, TimeUnit.MINUTES).toSeconds)
  val u: TimeUnit = SECONDS
  println(u eq TimeUnit.SECONDS)
  println(Duration(1.5, TimeUnit.HOURS).toMinutes)
