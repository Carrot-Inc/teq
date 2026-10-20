// lila: fishnet/Cleaner.scala:16 (`plies * Cleaner.timeoutPerPly`), tutor/TutorQueue.scala:89
// (`position * avgDuration`): when none of Int's own `*` overloads accepts the argument, the
// receiver is converted implicitly (duration's IntMult) and `*` is retried on the result.
import scala.concurrent.duration.*

object Cleaner:
  val timeoutPerPly = 7.seconds
  def analysisTimeout(plies: Int) = plies * timeoutPerPly + 3.seconds

@main def run() =
  println(Cleaner.analysisTimeout(20))
  val position = 3
  println((position * 2.minutes) / 2)
