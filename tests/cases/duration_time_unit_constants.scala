// scala-library's `duration.SECONDS` and the other units, declared as the singletons of the JDK's
// `TimeUnit` values, which the std stands in for with defs of its own `object TimeUnit`: their types
// widened to `TimeUnit` (lila's `LilaStream`, `outputDelay.toUnit(concurrent.duration.SECONDS)`).
import scala.concurrent.duration.*

@main def run(): Unit =
  val d = 1500.millis
  println(d.toUnit(concurrent.duration.SECONDS))
  println(d.toUnit(MILLISECONDS).toLong)
  println(Duration(2, MINUTES).toSeconds)
