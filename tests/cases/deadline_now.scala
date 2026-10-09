// `Deadline`: now, moved by a duration, the duration between two, and whether one has passed.
import scala.concurrent.duration.*

@main def run =
  val start = Deadline.now
  val later = start + 5.seconds
  println(later - start)
  println(later > start)
  println(later.hasTimeLeft())
  println((start - 1.second).isOverdue())
  println((Deadline.now - start).toMillis >= 0)
  println(List(later, start).min == start)
  println((later - 2.seconds) - start)
